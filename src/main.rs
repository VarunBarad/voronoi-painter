extern crate core;

use byteorder::{ByteOrder, LittleEndian};
use clap::{arg, Command};
use image::{GenericImageView, Rgba};
use rand::Rng;
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::fs::File;
use std::io::{Read, Write};
use std::panic;
use std::thread;

#[derive(Clone)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn squared_distance_from(&self, other_point: &Point) -> f64 {
        let horizontal_distance = (self.x - other_point.x).powf(2f64);
        let vertical_distance = (self.y - other_point.y).powf(2f64);

        horizontal_distance + vertical_distance
    }

    fn closest_anchor(
        &self,
        anchors: &Vec<Anchor>,
        minimum_distance_between_anchors: u32,
    ) -> Option<Anchor> {
        let x = (minimum_distance_between_anchors as f64) / 2f64;
        let x = x * x;

        let mut closest_anchor: Option<(Anchor, f64)> = None;
        for anchor in anchors {
            let distance = self.squared_distance_from(&anchor.point);
            if distance < x {
                closest_anchor = Some((anchor.clone(), distance));
            } else {
                match closest_anchor {
                    None => {
                        closest_anchor = Some((anchor.clone(), distance));
                    }
                    Some((_, min_distance)) => {
                        if min_distance > distance {
                            closest_anchor = Some((anchor.clone(), distance));
                        }
                    }
                }
            }
        }

        closest_anchor.map(|(anchor, _)| anchor)
    }
}

#[derive(Clone)]
struct Anchor {
    point: Point,
    color: Rgba<u8>,
}

struct Bounds {
    width: u64,
    height: u64,
}

struct Distance {
    minimum: u32,
    maximum: u32,
}

fn random_point_at_certain_distance_from_given_point(
    source_point: &Point,
    distance: &Distance,
    bounds: &Bounds,
) -> Point {
    let mut rng = rand::thread_rng();

    let angle = rng.gen::<f64>() * (2f64 * PI);
    let actual_distance = (distance.minimum as f64)
        + (rng.gen::<f64>() * ((distance.maximum - distance.minimum) as f64));

    let point = Point {
        x: (actual_distance * angle.cos()) + source_point.x,
        y: (actual_distance * angle.sin()) + source_point.y,
    };

    let is_point_in_horizontal_bounds = (point.x > 0f64) && (point.x < (bounds.width as f64));
    let is_point_in_vertical_bounds = (point.y > 0f64) && (point.y < (bounds.height as f64));

    if is_point_in_horizontal_bounds && is_point_in_vertical_bounds {
        point
    } else {
        random_point_at_certain_distance_from_given_point(source_point, distance, bounds)
    }
}

fn generate_anchor_candidates(
    source_point: &Point,
    distance: &Distance,
    bounds: &Bounds,
) -> Vec<Point> {
    let mut candidates = Vec::with_capacity(25);

    for _ in 0..25 {
        candidates.push(random_point_at_certain_distance_from_given_point(
            source_point,
            distance,
            bounds,
        ));
    }

    candidates
}

fn generate_anchor_points(bounds: &Bounds, minimum_distance: u32) -> Vec<Point> {
    let mut rng = rand::thread_rng();

    let squared_minimum_distance = minimum_distance * minimum_distance;

    let mut final_anchors: Vec<Point> = Vec::new();
    let mut anchor_candidates: VecDeque<Point> = VecDeque::new();

    let first_anchor = Point {
        x: rng.gen::<f64>() * (bounds.width as f64),
        y: rng.gen::<f64>() * (bounds.height as f64),
    };

    final_anchors.push(first_anchor.clone());

    let distance = Distance {
        minimum: minimum_distance,
        maximum: minimum_distance * 2,
    };
    anchor_candidates.extend(generate_anchor_candidates(&first_anchor, &distance, bounds));

    loop {
        match anchor_candidates.pop_front() {
            None => {
                break;
            }
            Some(candidate) => {
                let mut is_valid_anchor = true;
                for anchor in &final_anchors {
                    if anchor.squared_distance_from(&candidate) < (squared_minimum_distance as f64)
                    {
                        is_valid_anchor = false;
                        break;
                    }
                }

                if is_valid_anchor {
                    final_anchors.push(candidate.clone());

                    match final_anchors.last() {
                        None => {}
                        Some(source) => {
                            anchor_candidates
                                .extend(generate_anchor_candidates(source, &distance, bounds));
                        }
                    }
                }
            }
        }
    }

    final_anchors
}

fn pixel_calculator(
    x: u32,
    image_height: u32,
    anchors: Vec<Anchor>,
    minimum_distance_between_anchors: u32,
) -> Vec<(Point, Rgba<u8>)> {
    let mut pixels: Vec<(Point, Rgba<u8>)> = Vec::with_capacity(image_height as usize);

    let mut filtered_anchors: Vec<Anchor> = Vec::with_capacity(anchors.len());

    for anchor in anchors {
        if (anchor.point.x > (((x as i64) - (minimum_distance_between_anchors as i64)) as f64))
            && (anchor.point.x < (((x as i64) + (minimum_distance_between_anchors as i64)) as f64))
        {
            filtered_anchors.push(anchor);
        }
    }

    for y in 0..image_height {
        let point = Point {
            x: x as f64,
            y: y as f64,
        };
        let closest_anchor =
            point.closest_anchor(&filtered_anchors, minimum_distance_between_anchors);
        match closest_anchor {
            None => {}
            Some(anchor) => {
                pixels.push((point, anchor.color));
            }
        }
    }

    println!("Finished processing column: {}", x);

    pixels
}

fn read_anchor_points_from_file(anchors_cache_path: &str) -> std::io::Result<Vec<Point>> {
    let mut anchor_points: Vec<Point> = Vec::new();

    let mut existing_anchor_file = File::open(anchors_cache_path)?;

    let mut buffer: [u8; 8] = [0; 8];

    loop {
        let x = match existing_anchor_file.read_exact(&mut buffer) {
            Ok(_) => LittleEndian::read_f64(&buffer),
            Err(_) => {
                break;
            }
        };
        let y = match existing_anchor_file.read_exact(&mut buffer) {
            Ok(_) => LittleEndian::read_f64(&buffer),
            Err(_) => {
                break;
            }
        };

        anchor_points.push(Point { x, y });
    }

    Ok(anchor_points)
}

fn write_anchor_points_to_file(
    anchor_points: Vec<Point>,
    anchors_cache_path: &str,
) -> std::io::Result<()> {
    let mut anchor_file = File::create(anchors_cache_path)?;

    let mut buffer = [0; 8];

    for anchor in anchor_points {
        LittleEndian::write_f64(&mut buffer, anchor.x);
        match anchor_file.write_all(&buffer) {
            Ok(_) => {}
            Err(_) => {}
        }
        LittleEndian::write_f64(&mut buffer, anchor.y);
        match anchor_file.write_all(&buffer) {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    Ok(())
}

fn main() {
    let arguments = Command::new("voronoi-painter")
        .version("0.1.0")
        .author("Varun Barad <varun@varunbarad.com>")
        .about("CLI tool to convert an image to its voronoi diagram")
        .args_override_self(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("painting")
                .about("Convert a painting to its voronoi diagram")
                .arg(arg!(-i --input <VALUE>).required(true))
                .arg(arg!(-o --output <VALUE>).required(true))
                .arg(arg!(-a --anchors <VALUE>).required(false)),
        )
        .get_matches();

    match arguments.subcommand() {
        Some(("painting", sub_matches)) => match sub_matches.value_of("input") {
            None => {
                eprintln!("Path to input image not provided, please use the `--input <VALUE>` arg");
            }
            Some(input_image_path) => match sub_matches.value_of("output") {
                None => {
                    eprint!("Path for output not provided, please use the `--output <VALUE>` arg");
                }
                Some(output_path) => {
                    let input_image = image::open(input_image_path).unwrap();

                    let (image_width, image_height) = input_image.dimensions();

                    let minimum_distance = 10u32;
                    let bounds = Bounds {
                        width: image_width as u64,
                        height: image_height as u64,
                    };

                    let anchor_points = match sub_matches.value_of("anchors") {
                        None => generate_anchor_points(&bounds, minimum_distance),
                        Some(anchors_cache_path) => {
                            match read_anchor_points_from_file(anchors_cache_path) {
                                Ok(existing_anchor_points) => existing_anchor_points,
                                Err(_) => {
                                    let anchor_points =
                                        generate_anchor_points(&bounds, minimum_distance);
                                    match write_anchor_points_to_file(
                                        anchor_points.clone(),
                                        anchors_cache_path,
                                    ) {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }

                                    anchor_points
                                }
                            }
                        }
                    };

                    let mut anchors: Vec<Anchor> = Vec::with_capacity(anchor_points.len());
                    for point in anchor_points {
                        let x = point.x as u32;
                        let y = point.y as u32;
                        anchors.push(Anchor {
                            point,
                            color: input_image.get_pixel(x, y),
                        });
                    }

                    println!("Generated {} anchor points", anchors.len());

                    let mut output_image_buffer =
                        image::ImageBuffer::new(image_width, image_height);

                    for step in (0..image_width).step_by(10) {
                        let mut thread_pool = Vec::with_capacity(10);
                        for x in 0..10 {
                            if (x + step) >= image_width {
                                break;
                            } else {
                                let loop_anchors = anchors.clone();
                                let handle = thread::spawn(move || {
                                    pixel_calculator(
                                        x + step,
                                        image_height,
                                        loop_anchors,
                                        minimum_distance,
                                    )
                                });

                                thread_pool.push(handle);
                            }
                        }

                        for thread in thread_pool {
                            match thread.join() {
                                Ok(pixels) => {
                                    for (coordinates, color) in pixels {
                                        output_image_buffer.put_pixel(
                                            coordinates.x as u32,
                                            coordinates.y as u32,
                                            color,
                                        );
                                    }
                                }
                                Err(message) => {
                                    panic::resume_unwind(message);
                                }
                            }
                        }
                    }

                    output_image_buffer.save(output_path).unwrap();
                }
            },
        },
        _ => {
            eprintln!("No known sub-command found");
        }
    }
}
