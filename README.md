# HTTP Typed

HTTP client supporting custom request and response types. Pass any type into a `send` function or method, and it will return a result of your desired response type. `send` handles request serialization, http messaging, and response deserialization.

To keep this crate simple, it is is oriented towards a specific but very common pattern. If your use case meets the following conditions, this crate will work for you:
1. request-response communication
2. async rust functions
3. communicate over http (uses reqwest under the hood)
4. http body is serialized as json
5. status codes outside the 200 range are considered errors
6. request and response types must be serializable and deserializable using serde
7. the path and HTTP method can be determined from the concrete rust type used for the request

## Usage

To use this library, your request and response types must implement serde::Serialize and serde::Deserialize, respectively.

To take full advantage of all library features, you can implement `Request` for each of your request types, instantiate a `Client`, and then you can simply invoke `Client::send` to send requests.

```rust
let client = Client::new("http://example.com");
let response = client.send(MyRequest::new()).await?;
```

If you don't want to implement Request or create a Client, the most manual and basic way to use this library is by using `send_custom`.

```rust
let my_response: MyResponse = send_custom(
    "http://example.com/path/to/my/request/",
    HttpMethod::Get,
    MyRequest::new()
)
.await?;
```

One downside of the `send_custom` (and `send`) *function* is that it instantiates a client for every request, which is expensive. To improve performance, you can use the `Client::send_custom` (and `Client::send`) *method* instead to re-use an existing client for every request.

```rust
let client = Client::default();
let my_response: MyResponse = client.send_custom(
    "http://example.com/path/to/my/request/",
    HttpMethod::Get,
    MyRequest::new()
)
.await?;
```

You may also prefer not to specify metadata about the request every time you send a request, since these things will likely be the same for every request of this type. Describe the request metadata in the type system by implementing the Request trait.

```rust
pub trait Request {
    type Response;
    fn method(&self) -> HttpMethod;
    fn path(&self) -> String;
}
```

This increases design flexibility and boilerplate. See the [API Client Design](#api-client-design) section below for an explanation.

If you do not control the crate with the request and response structs, you can implement any traits for them using the newtype pattern, or with a reusable generic wrapper struct.

After implementing this trait, you can use the send function and method, which requires the base url to be included, instead of the full url. All other information about how to send the request and response is implied by the type of the input. This still creates a client on every request, so the performance is not optimal if you are sending multiple requests.

```rust
let my_response = send("http://example.com", MyRequest::new()).await?;
// The type of my_response is determined by the trait's associated type.
// It does not need to be inferrable from the calling context.
return my_response.some_field
```

If you want to send multiple requests, or if you don't want to include the base url when calling `send`, instantiate a Client:

```rust
let client = Client::new("http://example.com");
let my_response = client.send(MyRequest::new()).await?;
```

You can also define request groups. This defines a client type that is explicit about exactly which requests it can handle. The code will not compile if you try to send a request with the wrong client.

```rust
request_group!(MyApi { MyRequest1, MyRequest2 });
```
```rust
let my_client = Client::<MyApi>::new("http://example.com");
let my_response1 = my_client.send(MyRequest1::new()).await?; // works
let other_response = my_client.send(OtherRequest::new()).await?; // does not compile
```
If you want to restrict the request group, but still want to include the url for every call to `send`, `MyClient` has a `send_to` method that can be used with the default client to specify the url at the call-site.
```rust
let my_client = Client::<MyApi>::default();
let my_response2 = my_client.send_to("http://example.com", MyRequest2::new()).await?; // works
let other_response = my_client.send_to("http://example.com", OtherRequest::new()).await?; // does not compile
```

The send_to method can also be used to insert a string after the base_url and before the Request path.

```rust
let my_client = Client::new("http://example.com");
let my_response = my_client.send_to("/api/v2", MyRequest::new()).await?;
```

## API Client Design
Normally, a you might implement a custom client struct to connect to an API, including a custom method for every request. In doing so, you've forced all dependents of the API to make a choice between two options:
1. use the specific custom client struct that was already implemented, accepting any issues with it.
2. implement a custom client from scratch, re-writing and maintaining all the details about each request, including what http method to use, what path to use, how to serialize/deserialized the message, etc.

Instead, you can describe the metadata through trait definitions for ultimate flexibility, without locking dependents into a client implementation or needing to implement any custom clients structs. Dependents of the API now have better options:
1. use the Client struct provided by http-typed, accepting any issues with it. This is easier for you to support because you don't need to worry about implementation details of sending requests in general. You can just export or alias the `Client` struct.
2. implement a custom client that can generically handle types implementing Request by using the data returned by their methods. This is easier for dependents because they don't need to write any request-specific code. The Request trait exposes that information without locking them into a client implementation. Only a single generic request handler is sufficient.


## Other use cases
If your use case does not meet some of the conditions 2-7 described in the introduction, you'll find my other crate useful, which individually generalizes each of those, allowing any of them to be individually customized with minimal boilerplate. It is currently a work in progress, but almost complete. This crate and that crate will be source-code-compatible, meaning the other crate can be used as a drop-in replacement of this one without changing any code, just with more customization available.
