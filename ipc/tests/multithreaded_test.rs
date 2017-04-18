extern crate jsonrpc_core;
extern crate jsonrpc_ipc_server;

extern crate miow;

#[cfg(test)]
mod multithreaded_test {
    use jsonrpc_core::*;
    use jsonrpc_core::futures::Future;
    use jsonrpc_core::futures::future::ok;
    use jsonrpc_ipc_server::Server;

    #[cfg(windows)]
    use miow::pipe::connect;
    #[cfg(not(windows))]
    use std::os::unix::net::UnixStream;

    use std::thread;
    use std::time::Duration;
    use std::io::{Read, Write};

    #[cfg(windows)]
    fn pipe_name() -> &'static str {
        "\\\\.\\pipe\\Foo\\Bar\\Baz"
    }
    #[cfg(not(windows))]
    fn pipe_name() -> &'static str {
        "/tmp/foobar.sock"
    }

    #[cfg(windows)]
    fn say_to_pipe(pipe_name: &str, message: String) -> String {
        let mut connection =
            connect(pipe_name).expect("Failed to get a client connection to the pipe");
        connection
            .write_all(message.as_bytes())
            .expect("Failed to write to the pipe");

        let mut buf = [0u8; 1024];
        connection
            .read(&mut buf)
            .expect("Failed to read from the pipe");
        String::from_utf8_lossy(&buf)
            .into_owned()
            .trim_right_matches('\u{0}')
            .to_string()
    }
    #[cfg(not(windows))]
    fn say_to_pipe(pipe_name: &str, message: String) -> String {
        let mut connection =
            UnixStream::connect(pipe_name).expect("Failed to connect to unix socket");
        connection
            .write_all(message.as_bytes())
            .expect("Failed to write to the pipe");

        let mut buf = [0u8; 1024];
        connection
            .read(&mut buf)
            .expect("Failed to read from the pipe");
        String::from_utf8_lossy(&buf)
            .into_owned()
            .trim_right_matches('\u{0}')
            .to_string()
    }


    fn message(n: i32) -> String {
        format!(r#"{{ "jsonrpc":"2.0", "method":"hello", "params": {{"message": "Hello from {n}!"}}, "id": {n} }}"#, n=n)
    }

    fn expected_response(n: i32) -> String {
        format!(r#"{{"jsonrpc":"2.0","result":"hello accepted","id":{n}}}"#,
                n = n)
    }

    #[test]
    fn processes_several_requests_at_once() {
        let mut io = IoHandler::new();
        io.add_async_method("hello", |_params| {
            ok(String::new())
                .and_then(|x| {
                              thread::sleep(Duration::from_millis(100));
                              ok(x)
                          })
                .and_then(|_| ok(Value::String("hello accepted".into())))
                .boxed()
        });

        let server = Server::new(pipe_name(), io).unwrap();
        thread::spawn(move || server.run());

        thread::sleep(Duration::from_millis(100)); // Let's make sure the pipe server has been initialized

        let thread1 = thread::spawn(|| {
                                        assert_eq!(say_to_pipe(pipe_name(), message(1)),
                                                   expected_response(1))
                                    });
        let thread2 = thread::spawn(|| {
                                        assert_eq!(say_to_pipe(pipe_name(), message(2)),
                                                   expected_response(2))
                                    });

        thread1.join().unwrap();
        thread2.join().unwrap();
    }
}
