use crate::prelude::*;
use serde::Deserialize;

pub struct ImgurProvider;

impl Provider for ImgurProvider {
    fn try_get_accessor(&self, url: &Url) -> Option<Box<dyn Accessor>> {
        match url.domain()? {
            // An imgur album link has an /a/ in its route
            "imgur.com" if url.path().starts_with("/a/") => {
                Some(Box::new(ImgurAlbumAccessor::new(last_segment(url)?)))
            }

            // An imgur gallery link (don't ask what the difference is) uses /gallery/ instead
            "imgur.com" if url.path().starts_with("/gallery/") => {
                Some(Box::new(ImgurGalleryAccessor::new(last_segment(url)?)))
            }

            // A plain old image link has nothing but its hash
            "imgur.com" => Some(Box::new(ImgurSingleImageAccessor::new(last_segment(url)?))),

            // Annnnnd it looks like you didn't give us an imgur url at all!
            _ => None,
        }
    }

    fn configure_client(&self, builder: ClientBuilder) -> Client {
        use dotenv_codegen::dotenv;
        use reqwest::header::{self, HeaderValue};

        let mut headers = header::HeaderMap::new();

        headers.insert(header::ACCEPT, HeaderValue::from_static("text/json"));
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("imgrab 0.1.4+"),
        );
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Client-ID {}", dotenv!("imgur_client_id"))).unwrap(),
        );

        builder
            .default_headers(headers)
            .timeout(None)
            .build()
            .unwrap()
    }
}

pub struct ImgurSingleImageAccessor {
    id: String,
    is_complete: bool,
}

impl ImgurSingleImageAccessor {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            is_complete: false,
        }
    }
}

impl Accessor for ImgurSingleImageAccessor {
    fn next_page(&mut self, client: &Client) -> Result<VecDeque<String>> {
        use std::iter;

        if self.is_complete {
            return Ok(VecDeque::new());
        }

        let response: Response<Image> = client
            .get(&format!("https://api.imgur.com/3/image/{}", self.id,))
            .send()?
            .json()?;

        self.is_complete = true;
        Ok(iter::once(response.data.link).collect())
    }
}

pub struct ImgurAlbumAccessor {
    id: String,
    is_complete: bool,
}

impl ImgurAlbumAccessor {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            is_complete: false,
        }
    }
}

impl Accessor for ImgurAlbumAccessor {
    fn next_page(&mut self, client: &Client) -> Result<VecDeque<String>> {
        if self.is_complete {
            return Ok(VecDeque::new());
        }

        let response: Response<VecDeque<Image>> = client
            .get(&format!("https://api.imgur.com/3/album/{}/images", self.id,))
            .send()?
            .json()?;

        self.is_complete = true;
        Ok(response.data.into_iter().map(|x| x.link).collect())
    }
}

pub struct ImgurGalleryAccessor {
    id: String,
    is_complete: bool,
}

impl ImgurGalleryAccessor {
    fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            is_complete: false,
        }
    }
}

impl Accessor for ImgurGalleryAccessor {
    fn next_page(&mut self, client: &Client) -> Result<VecDeque<String>> {
        if self.is_complete {
            return Ok(VecDeque::new());
        }

        let response: Response<Gallery> = client
            .get(&format!(
                "https://api.imgur.com/3/gallery/album/{}",
                self.id,
            ))
            .send()?
            .json()?;

        self.is_complete = true;
        Ok(response.data.images.into_iter().map(|x| x.link).collect())
    }
}

fn last_segment(url: &Url) -> Option<&str> {
    url.path_segments().into_iter().flatten().last()
}

#[derive(Clone, Debug, Deserialize)]
struct Response<T> {
    data: T,
    success: bool,
    status: i16,
}

#[derive(Clone, Debug, Deserialize)]
struct Gallery {
    id: String,
    title: String,
    link: String,
    images: VecDeque<Image>,
}

#[derive(Clone, Debug, Deserialize)]
struct Image {
    id: String,
    width: u32,
    height: u32,
    size: u64,
    link: String,
}
