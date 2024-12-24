use clap::{builder::styling::RgbColor, ArgAction, Parser, Subcommand};
use image::*;
use std::path::Path;

use ndarray::{self, Array, Array3};
use video_rs::decode::Decoder;
use video_rs::encode::{Encoder, Settings};
use video_rs::time::Time;

use imgfx::*;

#[derive(Subcommand)]
enum SubCommands {
    Or {
        color: String,
    },
    And {
        color: String,
    },
    Xor {
        color: String,
    },
    Left {
        bits: String,
        raw: Option<String>,
    },
    Right {
        bits: String,
        raw: Option<String>,
    },
    Add {
        color: String,
    },
    Sub {
        color: String,
        raw: Option<String>,
    },
    Mult {
        color: String,
    },
    Pow {
        color: String,
    },
    Div {
        color: String,
    },
    Average {
        color: String,
    },
    Screen {
        color: String,
    },
    Overlay {
        color: String,
    },
    Bloom {
        intensity: f32,
        radius: f32,
        min_threshold: u8,
        max_threshold: Option<u8>,
    },
    Sort {
        direction: imgfx::sort::Direction,
        sort_by: imgfx::sort::SortBy,
        min_threshold: f32,
        max_threshold: f32,
    },
}

#[derive(Parser)]
#[command(name = "vidfx")]
#[command(version = "0.0.2")]
#[command(about = "Implementation of imgfx for videos", long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: SubCommands,

    /// path/to/input/image
    #[arg(short, long)]
    input: String,

    /// path/to/output/image
    #[arg(long, default_value = ".")]
    output: Option<String>,

    #[arg(short, long, default_value = "default")]
    visualization: String,

    #[arg(short, long)]
    bpm: Option<u32>,

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
    Osc { bpm: u32, wave_type: WaveType },
}

fn bpm_scale_factor(bpm: u32, wave_type: &WaveType, current_time: f64) -> f64 {
    let beat_duration = 60.0 / bpm as f64;
    let beat_progress = (current_time % beat_duration) / beat_duration;

    match wave_type {
        WaveType::Sine => (beat_progress * std::f64::consts::PI * 2.0).sin() * 0.5 + 0.5,
        WaveType::Saw => (1f64 - beat_progress) as f64,
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
    frame_width: u32,
    frame_height: u32,
    visualization_mode: VisualizationMode,
) -> Vec<RgbaImage>
where
    F: Fn(DynamicImage, f64) -> DynamicImage,
{
    let mut processed = vec![];
    let mut current_time = 0.0;

    for frame in decoder.decode_iter() {
        if let Ok((_, frame)) = frame {
            let scale_factor = match &visualization_mode {
                VisualizationMode::Default => 1.0,
                VisualizationMode::Osc { bpm, wave_type } => {
                    bpm_scale_factor(*bpm, wave_type, current_time)
                }
            };

            let rgb = frame
                .slice(ndarray::s![.., .., 0..3])
                .to_slice()
                .expect("Failed to slice frame into rgb array");

            let img = ImageBuffer::from_raw(frame_width, frame_height, rgb.to_vec())
                .expect("Failed to convert ndarray to ImageBuffer");

            let processed_frame = frame_processor(DynamicImage::ImageRgb8(img), scale_factor);

            let output = processed_frame.into_rgba8();

            processed.push(output);

            current_time = current_time + 1.0 / frame_rate as f64;
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
        SubCommands::Or { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            or(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::And { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            and(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::Xor { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            xor(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
                negate,
            )
        }
        SubCommands::Add { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            add(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Sub { color, raw } => {
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
        SubCommands::Mult { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            mult(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Pow { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            pow(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Div { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            div(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Left { bits, raw } | SubCommands::Right { bits, raw } => {
            let raw_flag = matches!(raw.as_deref(), Some("raw"));
            let direction = if matches!(cmd, SubCommands::Left { .. }) {
                BitshiftDirection::LEFT
            } else {
                BitshiftDirection::RIGHT
            };
            let bit_shift = bits.parse::<u8>().expect("Could not parse bits arg to u8");
            bitshift(img, direction, lhs.clone(), bit_shift, raw_flag)
        }
        SubCommands::Average { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            average(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Screen { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            screen(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Overlay { color } => {
            let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");
            overlay(
                img,
                lhs.clone(),
                rhs.clone(),
                scaled_color(rgb, scale_factor),
            )
        }
        SubCommands::Bloom {
            intensity,
            radius,
            min_threshold,
            max_threshold,
        } => imgfx::bloom(img, *intensity, *radius, *min_threshold, *max_threshold),

        SubCommands::Sort {
            direction,
            sort_by,
            min_threshold,
            max_threshold,
        } => sort(
            Into::into(img),
            *direction,
            *sort_by,
            *min_threshold * scale_factor as f32,
            *max_threshold * scale_factor as f32,
        ),
    }
}

fn main() {
    let args = Args::parse();

    let in_path = args.input;
    let out_path = args.output.unwrap_or("output.mp4".to_string());
    let negate = args.negate;

    video_rs::init().expect("Failed to init video_rs");
    let mut decoder =
        video_rs::Decoder::new(Path::new(&in_path)).expect("Failed to create decoder");

    let (width, height) = decoder.size();
    let frame_rate = decoder.frame_rate();

    let bpm = args.bpm;

    let visualization_mode = match args.visualization.as_str() {
        "default" => VisualizationMode::Default,
        "sine" => VisualizationMode::Osc {
            bpm: bpm.expect("No --bpm provided!"),
            wave_type: WaveType::Sine,
        },
        "saw" => VisualizationMode::Osc {
            bpm: bpm.expect("No --bpm provided!"),
            wave_type: WaveType::Saw,
        },
        "square" => VisualizationMode::Osc {
            bpm: bpm.expect("No --bpm provided!"),
            wave_type: WaveType::Square,
        },
        "triangle" => VisualizationMode::Osc {
            bpm: bpm.expect("No --bpm provided!"),
            wave_type: WaveType::Triangle,
        },
        _ => panic!("Unknown visualization mode"),
    };

    let processed = process_video(
        &mut decoder,
        |img, scale_factor| {
            DynamicImage::ImageRgba8(process_subcommand(
                &args.cmd,
                img,
                &args.lhs,
                &args.rhs,
                negate,
                scale_factor,
            ))
        },
        frame_rate as f64,
        width,
        height,
        visualization_mode,
    );

    let settings = Settings::preset_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder =
        Encoder::new(Path::new(&out_path), settings).expect("Failed to create encoder");

    let mut position = Time::zero();

    let frame_interval = (1.0 / frame_rate) as f64;

    for frame in processed {
        let rgb_image = rgba_to_rgb(&frame);

        encoder
            .encode(&image_to_ndarray(&rgb_image), position)
            .expect("Failed to encode frame");

        position = Time::from_secs_f64(position.as_secs_f64() + frame_interval);
    }
}

fn image_to_ndarray(image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Array3<u8> {
    let (width, height) = image.dimensions();
    Array::from_shape_vec(
        (height as usize, width as usize, 3),
        image.clone().into_raw(),
    )
    .expect("Failed to convert image to ndarray")
}

fn rgba_to_rgb(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let rgb_data: Vec<u8> = image
        .pixels()
        .flat_map(|p| {
            let [r, g, b, _a] = p.0;
            vec![r, g, b]
        })
        .collect();

    ImageBuffer::from_raw(width, height, rgb_data).expect("Failed to convert RGBA to RGB")
}
