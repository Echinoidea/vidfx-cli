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
    #[arg(short, long, global = true)]
    input: Option<String>,

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

fn main() {
    let args = Args::parse();

    let mut bit_shift = "";

    let in_path = args.input;
    let lhs = args.lhs;
    let rhs = args.rhs;
    let negate = args.negate;

    let vid_path = "/home/gabriel/Videos/vidfx/cs-short.mp4";

    video_rs::init().expect("Failed to init video_rs");

    let mut decoder =
        video_rs::Decoder::new(Path::new(vid_path)).expect("Failed to create decoder");

    let (width, height) = decoder.size();
    let frame_rate = decoder.frame_rate();

    let max_duration = 20.0;
    let max_frames = (frame_rate * max_duration).ceil() as usize;

    let mut frame_count = 0;
    let mut elapsed_time = 0.0;

    let mut processed: Vec<RgbaImage> = vec![];

    for frame in decoder.decode_iter() {
        if let Ok((_, frame)) = frame {
            if elapsed_time > max_duration {
                break;
            }

            let rgb = frame
                .slice(ndarray::s![.., .., 0..3])
                .to_slice()
                .expect("Failed to slice frame into rgb array");

            let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_raw(width, height, rgb.to_vec())
                    .expect("Failed to convert ndarray to ImageBuffer");

            processed.push(imgfx::mult(
                DynamicImage::from(img),
                None,
                None,
                RgbColor(20, 0, 0),
            ));

            frame_count += 1;
            elapsed_time += 1.0 / frame_rate;
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
