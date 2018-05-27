# serverless-rs

Proof of concept "javascript as a service" written in rust using the [v8-rs](https://github.com/dflemstr/v8-rs) library. This is not a complete application.

* Call javascript functions from rust `handler`
* Expose rust functions as javascript functions `response.addHeader`

## Create Handler

```
curl -X POST http://localhost:8088/v1/lambda/hello \
	-H"Content-Type: application/javascript" \
	-d 'function handler(request) { response.status = 418; response.addHeader("X-Hello-World", "true"); return "Hello " + request.getHeader("Host") + "!"; };'

>>>

{"id":1,"path":"/hello","hostname":"localhost:8088","code":"function handler(request) { response.status = 418; response.addHeader(\"X-Hello-World\", \"true\"); return \"Hello \" + request.host + \"!\"; };"}
```

## Usage

```
curl http://localhost:8088/hello

>>>

< HTTP/1.1 418 I'm a teapot
< content-length: 21
< x-hello-world: true
< date: Sat, 26 May 2018 07:30:16 GMT
< 
* Connection #0 to host localhost left intact
Hello localhost:8088!
```
