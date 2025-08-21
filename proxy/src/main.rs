mod maimai;

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Method, Error};
use hyper::service::{make_service_fn, service_fn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::Write;
use reqwest::redirect::Policy;
use crate::maimai::{get_open_url, maimai_handle};
use anyhow::{anyhow, Result};

async fn handle_proxy_request(mut req: Request<Body>) -> Result<Response<Body>> {
    if Method::CONNECT == req.method() {
        let host_with_port = req.uri().authority().map(|a| a.to_string()).unwrap_or_default();

        if host_with_port.is_empty() {
            return Ok(Response::builder().status(400).body(Body::from("CONNECT request missing authority")).unwrap());
        }
        tokio::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(mut client_stream) => {
                    println!("[CONNECT] 隧道请求建立，目标: {}", host_with_port);

                    let mut buffer = [0; 4096];
                    let n = match client_stream.read(&mut buffer).await {
                        Ok(0) | Err(_) => return ,
                        Ok(n) => n,
                    };

                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut req = httparse::Request::new(&mut headers);

                    if req.parse(&buffer[..n]).is_ok() {
                        if let (Some(method_str), Some(path)) = (req.method, req.path) {
                            let host_name = host_with_port.split(':').next().unwrap_or(&host_with_port);
                            let full_url = format!("https://{}{}", host_name, path);
                            println!("path:{path}");
                            if path=="/wc_auth/oauth/authorize/maimai-dx" {
                                match get_open_url(&full_url).await {
                                    Ok(location) => {
                                        // 拼 302 响应
                                        let response = format!(
                                            "HTTP/1.1 302 Found\r\n\
            Location: {}\r\n\
            Content-Length: 0\r\n\
            Connection: close\r\n\
            \r\n",
                                            location
                                        );
                                        client_stream.write_all(response.as_bytes()).await.unwrap();
                                    }
                                    Err(e) => {
                                        let html_body = format!(r#"<!DOCTYPE html><html lang="en">
<head><meta charset="UTF-8"><title>Error</title></head>
<body><h1>代理错误</h1><p>{}</p></body>
</html>"#, e);

                                        let response = format!(
                                            "HTTP/1.1 502 Bad Gateway\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            \r\n\
            {}",
                                            html_body.len(),
                                            html_body
                                        );
                                        client_stream.write_all(response.as_bytes()).await.unwrap();
                                    }
                                }
                            }else{
                                let res= maimai_handle(full_url,req.headers).await;
                                match res {
                                    Ok(login)=>{
                                        let redirect_url = format!(
                                            "http://192.168.0.16:5173/home?user_id={}&open_game_id={}&session_id={}&user_play_flag={}&new_user_id_flag={}&open_game_id_flag={}",
                                            login.user_id,
                                            login.open_game_id,
                                            login.session_id,
                                            login.user_play_flag,
                                            login.new_user_id_flag,
                                            login.open_game_id_flag
                                        );
                                        println!("{}", redirect_url);
                                        let response = format!(
                                            "HTTP/1.1 302 Found\r\n\
    Location: {}\r\n\
    Content-Length: 0\r\n\
    Connection: close\r\n\
    \r\n",
                                            redirect_url
                                        );
                                        client_stream.write_all(response.as_bytes()).await.unwrap();
                                        println!("---------------------------------------\n");
                                    },
                                    Err(e)=>{
                                        let html_body = format!(r#"<!DOCTYPE html><html lang="en">
                            <head>
                                <meta charset="UTF-8">
                                <title>MaiDaControl</title>
                            </head>
                            <body>
                                <h1>MaiDaControl Error:</h1>
                                <p>{:?}</p>
                            </body>
                            </html>"#,e);

                                        let response = format!( "HTTP/1.1 200 OK\r\n\
    Content-Type: text/html; charset=utf-8\r\n\
    Content-Length: {}\r\n\
    Connection: close\r\n\
    \r\n\
    {}",html_body.len(),html_body);
                                        client_stream.write_all(response.as_bytes()).await.unwrap();


                                    }
                                }
                            }


                        }
                    }
                },
                Err(e) => eprintln!("[CONNECT] 升级连接失败: {}", e),
            }
        });


        Ok(Response::builder().status(200).body(Body::empty()).unwrap())
    } else {
        Ok(Response::builder().status(400).body(Body::from("use Proxy pls.")).unwrap())
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 9854));
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_proxy_request))
    });
    let server = Server::bind(&addr).serve(make_svc);
    println!("Reqwest 转发代理正在监听 {}", addr);
    if let Err(e) = server.await {
        eprintln!("服务器错误: {}", e);
    }
}