use crate::prelude::*;
use std::collections::VecDeque;
use std::io::{self, Read};
use url::Url;

/// A media provider gives access to media from a certain source or source type.
///
/// In addition to providing validation for urls, the provider serves as a factory for any
/// associated media accessors.
trait Provider {
    fn is_valid(&self, url: &Url) -> bool;
    fn get_accessor(url: &Url) -> Box<dyn Accessor>;
}

/// An accessor provides access to media designated by the provider.
///
/// Generally, an accessor will fetch a page of media urls and this page will be processed before
/// another page is requested. This is done to improve compatibility with rate limits, and to
/// enable consumers to control when urls are consumed.
///
/// An accessor should also provide a properly-configured client for use by the consumer.
trait Accessor {
    fn client(&self) -> &Client;
    fn get_page(&mut self) -> Option<Result<Vec<String>>>;
}

struct TaskProvider<T> {
    accessor: T,
    current: VecDeque<String>,
}

impl<T: Accessor> TaskProvider<T> {
    fn new(accessor: T) -> Self {
        TaskProvider {
            accessor,
            current: VecDeque::new(),
        }
    }
}

struct Task {
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
            Some(url) => match self.accessor.client().get(&url).send() {
                Ok(response) => {
                    let disposition = response.headers().get(CONTENT_DISPOSITION).and_then(|h| {
                        let disposition = h.to_str().ok()?;
                        disposition
                            .rfind("filename=")
                            .map(|idx| disposition[(idx + 9)..].to_owned())
                    });

                    Some(Ok(Task {
                        response,
                        context: NamingContext {
                            url,
                            disposition,
                        }
                    }))
                }
                Err(e) => return Some(Err(e.into())),
            },

            None => match self.accessor.get_page()? {
                Err(e) => return Some(Err(e)),
                Ok(page) => {
                    self.current = page.into();
                    self.next()
                }
            },
        }
    }
}
