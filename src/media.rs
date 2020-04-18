pub mod imgur;

use crate::prelude::*;
use reqwest::blocking::ClientBuilder;
use std::collections::VecDeque;
use std::io::{self, Read};
use url::Url;

static USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_2) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/71.0.3578.98 Safari/537.36";

/// A media provider gives access to media from a certain source or source type.
///
/// In addition to providing validation for urls, the provider serves as a factory for any
/// associated media accessors.
pub trait Provider {
    fn try_get_accessor(&self, url: &Url) -> Option<Box<dyn Accessor>>;

    fn configure_client(&self, builder: ClientBuilder) -> Client {
        builder.user_agent(USER_AGENT).build().unwrap()
    }
}

/// An accessor provides access to media designated by the provider.
///
/// Generally, an accessor will fetch a page of media urls and this page will be processed before
/// another page is requested. This is done to improve compatibility with rate limits, and to
/// enable consumers to control when urls are consumed.
pub trait Accessor {
    fn next_page(&mut self, client: &Client) -> Result<VecDeque<String>>;
}

pub struct TaskProvider<T> {
    client: Client,
    accessor: T,
    current: VecDeque<String>,
}

impl<T: Accessor> TaskProvider<T> {
    pub fn new(accessor: T, client: Client) -> Self {
        TaskProvider {
            client,
            accessor,
            current: VecDeque::new(),
        }
    }
}

pub struct Task {
    response: Response,
    context: NamingContext,
}

struct NamingContext {
    url: String,
    disposition: Option<String>,
}

impl Read for Task {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.response.read(b)
    }
}

impl<T: Accessor> Iterator for TaskProvider<T> {
    type Item = Result<Task>;

    fn next(&mut self) -> Option<Self::Item> {
        use reqwest::header::CONTENT_DISPOSITION;

        match self.current.pop_front() {
            Some(url) => match self.client.get(&url).send() {
                Ok(response) => {
                    let disposition = response.headers().get(CONTENT_DISPOSITION).and_then(|h| {
                        let disposition = h.to_str().ok()?;
                        disposition
                            .rfind("filename=")
                            .map(|idx| disposition[(idx + 9)..].to_owned())
                    });

                    Some(Ok(Task {
                        response,
                        context: NamingContext { url, disposition },
                    }))
                }
                Err(e) => return Some(Err(e.into())),
            },

            None => match self.accessor.next_page(&self.client) {
                Err(e) => return Some(Err(e)),
                Ok(page) => {
                    self.current = page;
                    self.next()
                }
            },
        }
    }
}
