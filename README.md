# HTTP Typed

Http Client implementation supporting custom request and response types. Pass any type into a `send` functions or method, and it will return a result of your desired type. `send` handles request serialization, http messaging, and response deserialization.

To keep this crate simple, it is is oriented towards a specific but very common pattern. If your use case meets the following conditions, this crate will work for you:
1. request-response communication
2. async rust functions
3. communicate over http (uses reqwest under the hood)
4. http body is serialized as json
5. status codes outside the 200 range are considered errors
6. request and response types must be serializable and deserializable using serde
7. the path and HTTP method can be determined from the concrete rust type used for the request

## Usage

If the request and response types already implement serde::Serialize and serde::Deserialize respectively, the most manual and basic way to use this library is by using `send_custom`:

```rust
let my_response: MyResponse = send_custom(
    "http://example.com/path/to/my/request/",
    HttpMethod::Get,
    MyRequest::new()
)
.await?;
```

The downside to `send_custom` is that you must specify metadata about the request every time you send a request, even though these things will likely be the same for every request of this type. To avoid this, you may describe the request metadata in the type system by implementing the Request trait.

```rust
pub trait Request {
    type Response;
    fn method(&self) -> HttpMethod;
    fn path(&self) -> String;
}
```

This increases design flexibility and can reduce boilerplate. Normally, you might implement a custom gateway struct for every api you interact with, writing a custom send method for every request. Instead, if the metadata is described through the type system, there is no need for custom client structs.

If you do not control the crate with the request and response structs, you can implement any traits for them using the newtype pattern, or with a reusable generic wrapper struct.

After implementing this trait, you can use the send function, which requires the base url to be included, instead of the full url. All other information about how to send the request and response is implied by the type of the input.

```rust
let my_response = send("http://example.com", MyRequest::new()).await?;
// The type of my_response is determined by the trait's associated type.
// It does not need to be inferrable from the calling context.
return my_response.some_field
```

If you don't want to include the base url with every call to `send`, instantiate a Client:

```rust
let client = Client::new("http://example.com");
let my_response = client.send(MyRequest::new()).await?;
```

You can also define request groups. This defines a client type that is explicit about exactly which requests it can handle. The code will not compile if you try to send a request with the wrong client. If you want to restrict the request group, but still want to include the url for every call to `send`, `MyClient` has a `send_to` method that can be used without instantiating the struct.

```rust
request_group!(MyApi { MyRequest1, MyRequest2 });

let my_client = Client::<MyApi>::new("http://example.com");
let my_response1 = my_client.send(MyRequest1::new()).await?; // works
let other_response = my_client.send(OtherRequest::new()).await?; // blocked at compile time

type MyClient = Client::<MyApi>;
let my_response2 = MyClient::send_to("http://example.com", MyRequest2::new()).await?; // works
let other_response = MyClient::send_to("http://example.com", OtherRequest::new()).await?; // blocked at compile time
```

## Other use cases
If your use case does not meet some of the conditions 2-7 described in the introduction, you'll find my other crate useful, which individually generalizes each of those, allowing any of them to be individually customized with minimal boilerplate. It is currently a work in progress, but almost complete. This crate and that crate will be source-code-compatible, meaning the other crate can be used as a drop-in replacement of this one without changing any code, just with more customization available.
