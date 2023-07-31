use std::marker::PhantomData;

pub trait Request {
    /// Type to deserialize from the http response body
    type Response;
    /// HTTP method that the request will be sent with
    fn method(&self) -> HttpMethod;
    /// String to appended to the end of url when sending this request.
    fn path(&self) -> String;
}

/// A client to delegate to the send function that provides the ability to
/// optionally specify:
/// - a base url to be used for all requests
/// - a request group to constrain the request types accepted by this type
#[derive(Debug, Clone)]
pub struct Client<RequestGroup = All> {
    base_url: String,
    _p: PhantomData<RequestGroup>,
}

impl<RequestGroup> Client<RequestGroup> {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            _p: PhantomData,
        }
    }

    /// Send the provided request to the host at the specified base url, using
    /// the request metadata specified by the Request implementation. This
    /// method upgrades from the `send` function by enabling you to constrain
    /// the request group with the type system.
    pub async fn send_to<Req>(base_url: &str, request: Req) -> Result<Req::Response, Error>
    where
        Req: Request + serde::Serialize + InRequestGroup<RequestGroup>,
        Req::Response: for<'a> serde::Deserialize<'a>,
    {
        send(base_url, request).await
    }

    /// Send the provided request to the host at the specified base url, using
    /// the request metadata specified by the Request implementation. This
    /// method upgrades from the `send_to` method by allowing you specify the
    /// base url at the time of instantiation rather than passing it to every
    /// `send` call.
    pub async fn send<Req>(&self, request: Req) -> Result<Req::Response, Error>
    where
        Req: Request + serde::Serialize + InRequestGroup<RequestGroup>,
        Req::Response: for<'a> serde::Deserialize<'a>,
    {
        send(&self.base_url, request).await
    }
}

/// Send the provided request to the host at the specified base url, using the
/// request metadata specified by the Request implementation.
pub async fn send<Req>(base_url: &str, request: Req) -> Result<Req::Response, Error>
where
    Req: Request + serde::Serialize,
    Req::Response: for<'a> serde::Deserialize<'a>,
{
    let url = join_url(base_url, request.path());
    send_custom(&url, request.method(), request).await
}

/// Send the provided request to the host at the specified base url using the
/// specified method, and deserialize the response as the specified response
/// type
pub async fn send_custom<Req, Res>(
    url: &str,
    method: HttpMethod,
    request: Req,
) -> Result<Res, Error>
where
    Req: serde::Serialize,
    Res: for<'a> serde::Deserialize<'a>,
{
    let response = reqwest::Client::new()
        .request(method.into(), url)
        .body(serde_json::to_string(&request)?.into_bytes())
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        let body = response.bytes().await?;
        Ok(serde_json::from_slice(&body)?)
    } else {
        let message = match response.bytes().await {
            Ok(bytes) => match std::str::from_utf8(&bytes) {
                Ok(message) => message.to_owned(),
                Err(e) => format!("failed to parse body: {e:?}"),
            },
            Err(e) => format!("failed to get body: {e:?}"),
        };
        Err(Error::InvalidStatusCode(status.into(), message))
    }
}

/// Define a request group to constrain which requests can be used with a client.
/// ```ignore
/// request_group!(MyApi { MyRequest1, MyRequest2 });
/// ```
#[macro_export]
macro_rules! request_group {
    ($viz:vis $Name:ident { $($Request:ident),*$(,)? }) => {
        $viz struct $Name;
        $(impl $crate::InRequestGroup<$Name> for $Request {})*
    };
}

/// Indicates that a request is part of a request group. If you use the
/// request_group macro to define the group, it will handle the implementation
/// of this trait automatically.
pub trait InRequestGroup<Group> {}

/// The default group. All requests are in this group.
pub struct All;
impl<T> InRequestGroup<All> for T {}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    ClientError(#[from] reqwest::Error),
    #[error("serde serialization error: {0}")]
    SerializationError(#[from] serde_json::error::Error),
    #[error("invalid status code {0} with response body: `{1}`")]
    InvalidStatusCode(u16, String),
}

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
}

impl From<HttpMethod> for reqwest::Method {
    fn from(value: HttpMethod) -> Self {
        match value {
            HttpMethod::Options => reqwest::Method::OPTIONS,
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Trace => reqwest::Method::TRACE,
            HttpMethod::Connect => reqwest::Method::CONNECT,
            HttpMethod::Patch => reqwest::Method::PATCH,
        }
    }
}

fn join_url(base_url: &str, path: String) -> String {
    if base_url.chars().last().map(|c| c == '/').unwrap_or(true)
        || path.chars().next().map(|c| c == '/').unwrap_or(true)
    {
        format!("{base_url}{}", path)
    } else {
        format!("{base_url}/{}", path)
    }
}
