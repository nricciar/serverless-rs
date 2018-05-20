# serverless-rs

Proof of concept "javascript as a service" written in rust using the [v8-rs](https://github.com/dflemstr/v8-rs) library. This is not a complete application.

* Call javascript functions from rust `handler`
* Expose rust functions as javascript functions `hello`

## Create Handler

```
curl -X POST http://localhost:8088/v1/lambda/hello_world \
	-H"Content-Type: application/json" \
	-d '{"javascript":"function handler(request) { return request.hostname + request.path + \" : Hello \" + hello(); };"}'

>>>

{"id":1,"path":"/hello_world","hostname":"localhost:8088","code":"function handler(request) { return request.hostname + request.path + \" : Hello \" + hello(); };"}
```

## Usage

```
curl http://localhost:8088/hello_world

>>>

localhost:8088/hello_world : Hello World!
```
