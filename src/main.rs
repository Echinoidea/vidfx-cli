use clap::{builder::styling::RgbColor, ArgAction, Parser, Subcommand};
use ffmpeg_next::{self as ffmpeg, encoder, format};
use image::*;
use std::io::{self, BufWriter, Cursor, Read, Write};

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
    let input_path = args.input.expect("Input video path is required.");
    let output_path = args.output.expect("Input video path is required.");

    // Initialize FFmpeg
    ffmpeg::init().expect("Failed to initialize FFmpeg");

    // Open input video
    let mut ictx = ffmpeg::format::input(&input_path).expect("Failed to open input video");
    let input_video_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .expect("Failed to find video stream");

    let video_index = input_video_stream.index();
    let codec_context =
        ffmpeg::codec::context::Context::from_parameters(input_video_stream.parameters())
            .expect("Failed to create codec context");

    let mut decoder = codec_context
        .decoder()
        .video()
        .expect("Failed to create decoder");

    // Set up output video
    let mut octx = format::output(&output_path).expect("Failed to create output context");
    let global_header = octx
        .format()
        .flags()
        .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);

    let mut stream = octx
        .add_stream(ffmpeg::codec::Id::H264)
        .expect("Failed to add stream");
    let codec_context = ffmpeg::codec::context::Context::new();

    let mut encoder = codec_context
        .encoder()
        .video()
        .expect("Failed to get video encoder");

    // Configure the encoder
    encoder.set_width(decoder.width());
    encoder.set_height(decoder.height());
    encoder.set_format(ffmpeg::format::Pixel::YUV420P); // Use a format supported by H.264
    encoder.set_time_base((1, 25)); // Set frame rate to 25 FPS

    if octx
        .format()
        .flags()
        .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER)
    {
        encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
    }

    // Assign encoder settings to the stream
    stream
        .set_parameters(encoder.parameters())
        .expect("Failed to set stream parameters");

    // Open the encoder
    encoder
        .open_as(ffmpeg::codec::Id::H264)
        .expect("Failed to open encoder");

    // Write header to output context
    octx.write_header().expect("Failed to write header");

    encoder.set_time_base((1, 25)); // Set frame rate (adjust as needed)
    if global_header {
        encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
    }

    let mut encoder = encoder.open_as(ffmpeg::codec::Id::H264).unwrap();
    octx.write_header().expect("Failed to write header");

    // Process frames
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_index {
            let mut decoded = ffmpeg::frame::Video::empty();
            if decoder.decode(&packet, &mut decoded).is_ok() {
                let mut frame = frame_to_image(&decoded);

                // Apply the selected effect
                frame = match args.cmd {
                    SubCommands::OR { ref color } => {
                        let rgb = hex_to_rgb(color).expect("Could not convert color to rgb");

                        // Convert input DynamicImage to RgbaImage
                        let frame_buffer = frame.to_rgba8();

                        // Process the frame using the `or` function
                        let result = or(
                            frame_buffer.into(),
                            args.lhs.clone(),
                            args.rhs.clone(),
                            RgbColor(rgb.0, rgb.1, rgb.2),
                            args.negate,
                        );

                        // Wrap the output back into a DynamicImage (if required)
                        image::DynamicImage::ImageRgba8(result)
                    }
                    SubCommands::AND { color } => todo!(),
                    SubCommands::XOR { color } => todo!(),
                    SubCommands::ADD { color } => todo!(),
                    SubCommands::SUB { color, raw } => todo!(),
                    SubCommands::MULT { color } => todo!(),
                    SubCommands::DIV { color } => todo!(),
                    SubCommands::AVG { color } => todo!(),
                    SubCommands::SCREEN { color } => todo!(),
                    SubCommands::OVERLAY { color } => todo!(),
                    SubCommands::LEFT { bits, raw } => todo!(),
                    SubCommands::RIGHT { bits, raw } => todo!(),
                    SubCommands::BLOOM {
                        intensity,
                        radius,
                        min_threshold,
                        max_threshold,
                    } => todo!(),
                };

                let processed_frame = image_to_frame(&frame);

                // Encode and write the frame
                let mut encoded = ffmpeg::Packet::empty();
                if encoder.encode(&processed_frame, &mut encoded).is_ok() {
                    encoded.set_stream(0); // Set the appropriate stream index
                    octx.interleaved_write_packet(&encoded)
                        .expect("Failed to write packet");
                }
            }
        }
    }

    octx.write_trailer().expect("Failed to write trailer");
}

// Helper functions to convert FFmpeg frames to/from ImageBuffer
fn frame_to_image(frame: &ffmpeg::frame::Video) -> DynamicImage {
    let mut buffer = ImageBuffer::new(frame.width(), frame.height());
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let data = frame.data(0); // Assume planar format
        let offset = (y * frame.width() + x) as usize * 4;
        *pixel = Rgba([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
    }
    DynamicImage::ImageRgba8(buffer)
}

fn image_to_frame(image: &DynamicImage) -> ffmpeg::frame::Video {
    let rgba = image.to_rgba8();
    let mut frame =
        ffmpeg::frame::Video::new(ffmpeg::format::Pixel::RGBA, rgba.width(), rgba.height());
    frame.plane_mut(0).copy_from_slice(&rgba.into_raw());
    frame
}
