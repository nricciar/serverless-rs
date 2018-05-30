Proof of concept "javascript as a service" written in rust using the [v8-rs](https://github.com/dflemstr/v8-rs) library. This is not a complete application.

## Create

```
curl -X POST http://localhost:8088/v1/lambda/hello \
-H"Content-Type: application/javascript" \
-d @- << EOF
function handler(request) {
  response.status = 418; 
  response.addHeader("X-Hello-World", "true"); 

  return "Hello " + request.getHeader("Host") + "!"; 
};
EOF

>>>

{"id":1,"path":"/hello","hostname":"localhost:8088","code":"function handler(request) {  response.status = 418;   response.addHeader(\"X-Hello-World\", \"true\");   return \"Hello \" + request.getHeader(\"Host\") + \"!\"; };"}
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

## JSON

```
curl -X POST http://localhost:8088/v1/lambda/hello \
-H"Content-Type: application/javascript" \
-d @- << EOF
function handler(request) {
  return "Hello " + request.json().name + "!"; 
};
EOF
```

### Usage

```
curl -v -X POST http://localhost:8088/hello \
	-d '{"name":"John"}'

>>>

< HTTP/1.1 200 OK
< content-length: 11
< date: Wed, 30 May 2018 05:49:16 GMT
< 
* Connection #0 to host localhost left intact
Hello John!
```

## HTTP Request

```
curl -X POST http://localhost:8088/v1/lambda/http \
-H"Content-Type: application/javascript" \
-d @- << EOF
function handler(request) { 
  resp = http.request({uri: "http://samples.openweathermap.org/data/2.5/weather?q=London,uk&appid=b6907d289e10d714a6e88b30761fae22"}); 
  response.status = resp.status; 
  response.headers = resp.headers; 
  data = resp.json(); 

  return "Loc : " + data.name + " [" + data.sys.country + "]"; 
};
EOF
```
