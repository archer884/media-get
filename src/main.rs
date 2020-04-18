mod error;
mod media;
mod prelude;

use media::Provider;

fn main() {
    let registered_providers: Vec<Box<dyn Provider>> = vec![Box::new(media::imgur::ImgurProvider)];
}
