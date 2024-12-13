use clap::{builder::styling::RgbColor, ArgAction, Parser, Subcommand};
use image::*;
use std::{
    fmt::write,
    path::{Path, PathBuf},
};

use ndarray::{self, Array, Array3};
use video_rs::decode::Decoder;
use video_rs::encode::{Encoder, Settings};
use video_rs::time::Time;

use imgfx::*;

#[derive(Subcommand)]
#[allow(non_camel_case_types)]
enum SubCommands {
    OR {
        color: String,
    },
    AND {
        color: String,
    },
    XOR {
        color: String,
    },
    LEFT {
        bits: String,
        raw: Option<String>,
    },
    RIGHT {
        bits: String,
        raw: Option<String>,
    },
    ADD {
        color: String,
    },
    SUB {
        color: String,
        raw: Option<String>,
    },
    MULT {
        color: String,
    },
    DIV {
        color: String,
    },
    AVG {
        color: String,
    },
    SCREEN {
        color: String,
    },
    OVERLAY {
        color: String,
    },
    BLOOM {
        intensity: f32,
        radius: f32,
        min_threshold: u8,
        max_threshold: Option<u8>,
    },
}

#[derive(Parser)]
#[command(name = "img-mod")]
#[command(version = "0.2.0")]
#[command(about = "Arithmetic, logical, bitwise, filtering, and higher level operations for images.", long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: SubCommands,

    /// path/to/input/image
    #[arg(short, long)]
    input: String,

    /// path/to/output/image
    #[arg(long, default_value = ".")]
    output: Option<String>,

    /// Specify the left hand side operands for the function. E.g. --lhs b g r
    #[arg(long, num_args(1..), global = true)]
    lhs: Option<Vec<String>>,

    /// Specify the right hand side operands for the function. E.g. --rhs b r b
    #[arg(long, num_args(1..), global = true)]
    rhs: Option<Vec<String>>,

    /// If function is 'left' or 'right', how many bits to shift by.
    #[arg(short, long)]
    bit_shift: Option<u8>,

    /// Negate the logical operator
    #[arg(short, long, action=ArgAction::SetTrue, global = true)]
    negate: bool,
}

enum WaveType {
    Sine,
    Saw,
    Square,
    Triangle,
}

enum VisualizationMode {
    Default,
    Pulse { bpm: u32, wave_type: WaveType },
}

fn bpm_scale_factor(bpm: u32, wave_type: &WaveType, current_time: f64) -> f64 {
    let beat_duration = 60.0 / bpm as f64;
    let beat_progress = (current_time % beat_duration) / beat_duration;

    match wave_type {
        WaveType::Sine => (beat_progress * std::f64::consts::PI * 2.0).sin() * 0.5 + 0.5,
        WaveType::Saw => beat_progress,
        WaveType::Square => {
            if beat_progress < 0.5 {
                1.0
            } else {
                0.0
            }
        }
        WaveType::Triangle => 1.0 - (2.0 * beat_progress - 1.0).abs(),
    }
}

fn process_video<F>(
    decoder: &mut Decoder,
    frame_processor: F,
    frame_rate: f64,
    max_duration: f64,
    frame_width: u32,
    frame_height: u32,
    visualization_mode: VisualizationMode,
) -> Vec<RgbaImage>
where
    F: Fn(DynamicImage, f64) -> DynamicImage,
{
    let mut processed = vec![];
    let mut elapsed_time = 0.0;
    let mut current_time = 0.0;

    for frame in decoder.decode_iter() {
        if let Ok((_, frame)) = frame {
            if elapsed_time > max_duration {
                break;
            }

            let scale_factor = match &visualization_mode {
                VisualizationMode::Default => 1.0, // No BPM logic, use default scale
                VisualizationMode::Pulse { bpm, wave_type } => {
                    bpm_scale_factor(*bpm, wave_type, current_time)
                }
            };

            let rgb = frame
                .slice(ndarray::s![.., .., 0..3])
                .to_slice()
                .expect("Failed to slice frame into rgb array");

            let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_raw(frame_width, frame_height, rgb.to_vec())
                    .expect("Failed to convert ndarray to ImageBuffer");

            let output = frame_processor(DynamicImage::ImageRgb8(img), scale_factor);
            processed.push(output);

            elapsed_time += 1.0 / frame_rate;
            current_time += 1.0 / frame_rate;
        } else {
            break;
        }
    }

    processed
}

fn scaled_color(rgb: (u8, u8, u8), scale_factor: f64) -> RgbColor {
    RgbColor(
        (rgb.0 as f64 * scale_factor) as u8,
        (rgb.1 as f64 * scale_factor) as u8,
        (rgb.2 as f64 * scale_factor) as u8,
    )
}

fn process_subcommand(
    cmd: &SubCommands,
    img: DynamicImage,
    lhs: &Option<Vec<String>>,
    rhs: &Option<Vec<String>>,
    negate: bool,
    scale_factor: f64,
) -> RgbaImage {
    match cmd {
        SubCommands::OR { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            or(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::AND { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            and(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::XOR { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            xor(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::ADD { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            add(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::SUB { color, raw } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            let raw_flag = matches!(raw.as_deref(), Some("raw"));
            sub(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                raw_flag,
            )
        }
        SubCommands::MULT { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            mult(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::DIV { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            div(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::LEFT { bits, raw } | SubCommands::RIGHT { bits, raw } => {
            let raw_flag = matches!(raw.as_deref(), Some("raw"));
            let direction = if matches!(cmd, SubCommands::LEFT { .. }) {
                BitshiftDirection::LEFT
            } else {
                BitshiftDirection::RIGHT
            };
            let bit_shift = bits.parse::<u8>().expect("Could not parse bits arg to u8");
            bitshift(img, direction, lhs.clone(), bit_shift, raw_flag)
        }
        SubCommands::AVG { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            average(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::SCREEN { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            screen(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::OVERLAY { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            overlay(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::BLOOM {
            intensity,
            radius,
            min_threshold,
            max_threshold,
        } => imgfx::bloom(img, *intensity, *radius, *min_threshold, *max_threshold),
    }
}

fn main() {
    let args = Args::parse();

    let mut bit_shift = "";

    let in_path = args.input;
    let negate = args.negate;

    video_rs::init().expect("Failed to init video_rs");
    let mut decoder =
        video_rs::Decoder::new(Path::new(&in_path.to_string())).expect("Failed to create decoder");

    let (width, height) = decoder.size();
    let frame_rate = decoder.frame_rate();

    let max_duration = 20.0;
    let max_frames = (frame_rate * max_duration).ceil() as usize;

    let mut frame_count = 0;
    let mut elapsed_time = 0.0;

    let mut processed: Vec<RgbaImage> = vec![];

    // temp!
    let bpm = 180.0;
    let beat_duration = 60.0 / bpm;
    let mut current_time: f64 = 0.0;

    for frame in decoder.decode_iter() {
        if let Ok((_, frame)) = frame {
            let lhs = &args.lhs;
            let rhs = &args.rhs;

            if elapsed_time > max_duration {
                break;
            }

            // temp!
            let beat_progress = (current_time % beat_duration) / beat_duration;
            let scale_factor = 1.0 - beat_progress;

            let rgb = frame
                .slice(ndarray::s![.., .., 0..3])
                .to_slice()
                .expect("Failed to slice frame into rgb array");

            let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_raw(width, height, rgb.to_vec())
                    .expect("Failed to convert ndarray to ImageBuffer");

            let output = match args.cmd {
                SubCommands::OR { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    or(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(
                            (rgb.0 as f64 * scale_factor) as u8,
                            (rgb.1 as f64 * scale_factor) as u8,
                            (rgb.2 as f64 * scale_factor) as u8,
                        ),
                        negate,
                    )
                }
                SubCommands::AND { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    and(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                        negate,
                    )
                }
                SubCommands::XOR { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    xor(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                        negate,
                    )
                }
                SubCommands::ADD { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    add(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::SUB { ref color, ref raw } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");

                    let raw = match raw.as_deref() {
                        Some("raw") => true,
                        Some(_) => false,
                        None => false,
                    };

                    sub(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                        raw,
                    )
                }
                SubCommands::MULT { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    mult(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::DIV { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    div(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::LEFT { ref bits, ref raw } => {
                    bit_shift = &bits;

                    let raw = match raw.as_deref() {
                        Some("raw") => true,
                        Some(_) => false,
                        None => false,
                    };

                    bitshift(
                        DynamicImage::ImageRgb8(img),
                        BitshiftDirection::LEFT,
                        lhs.clone(),
                        bit_shift
                            .parse::<u8>()
                            .expect("Could not parse bits arg to u8"),
                        raw,
                    )
                }
                SubCommands::RIGHT { ref bits, ref raw } => {
                    bit_shift = &bits;

                    let raw = match raw.as_deref() {
                        Some("raw") => true,
                        Some(_) => false,
                        None => false,
                    };

                    bitshift(
                        DynamicImage::ImageRgb8(img),
                        BitshiftDirection::RIGHT,
                        lhs.clone(),
                        bit_shift
                            .parse::<u8>()
                            .expect("Could not parse bits arg to u8"),
                        raw,
                    )
                }
                SubCommands::AVG { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    average(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::SCREEN { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    screen(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::OVERLAY { ref color } => {
                    let rgb = hex_to_rgb(&color).expect("Could not convert color to rgb");
                    overlay(
                        DynamicImage::ImageRgb8(img),
                        lhs.clone(),
                        rhs.clone(),
                        RgbColor(rgb.0, rgb.1, rgb.2),
                    )
                }
                SubCommands::BLOOM {
                    intensity,
                    radius,
                    min_threshold,
                    max_threshold,
                } => imgfx::bloom(
                    DynamicImage::ImageRgb8(img),
                    intensity,
                    radius,
                    min_threshold,
                    max_threshold,
                ),
            };

            processed.push(output);

            frame_count += 1;
            elapsed_time += 1.0 / frame_rate;

            current_time = current_time + 1.0 / frame_rate as f64;
        } else {
            break;
        }
    }

    let settings = Settings::preset_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder =
        Encoder::new(Path::new("video-rs-out.mp4"), settings).expect("Failed to create encoder");

    let mut position = Time::zero();

    let duration: Time = Time::from_nth_of_a_second(max_duration as usize);

    for i in processed {
        let rgb_image = rgba_to_rgb(&i);

        encoder
            .encode(&image_to_ndarray(&rgb_image), position)
            .expect("Failed to encode frame");

        position = position.aligned_with(duration).add();
    }
}

fn image_to_ndarray(image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Array3<u8> {
    let (width, height) = image.dimensions();
    Array::from_shape_vec(
        (height as usize, width as usize, 3), // 3 channels for RGB
        image.clone().into_raw(),
    )
    .expect("Failed to convert image to ndarray")
}

fn rgba_to_rgb(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let rgb_data: Vec<u8> = image
        .pixels()
        .flat_map(|p| {
            let [r, g, b, _a] = p.0; // Ignore alpha channel
            vec![r, g, b]
        })
        .collect();

    ImageBuffer::from_raw(width, height, rgb_data).expect("Failed to convert RGBA to RGB")
}
