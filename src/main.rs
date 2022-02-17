use image::GenericImageView;

fn main() {
    let img = image::open("/Users/varunb/Desktop/painting-in.jpg").unwrap();

    println!("Dimensions {:?}", img.dimensions());

    println!("Color {:?}", img.color());

    img.save("/Users/varunb/Desktop/painting-out.png").unwrap();
}
