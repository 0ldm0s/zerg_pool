//! 精简版网络通信模块 - 仅实现Queen端ROUTER socket

use std::io::{self, Read, Write};
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use prost::Message;
use thiserror::Error;
use crate::Process;

use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};
use crate::RegistrationError;

/// 进程间消息枚举
#[derive(Debug)]
pub enum ProcessMessage {
    Registration(Registration),
    Heartbeat(Heartbeat),
}

/// 网络通信错误类型
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Registration error: {0}")]
    Registration(#[from] RegistrationError),
}

/// 网络通信核心结构体
pub struct HiveNetwork {
    listener: TcpListener,
    poll: Poll,
    connections: Vec<TcpStream>,
}

impl HiveNetwork {
    /// 获取当前TCP监听地址
    pub fn current_endpoint(&self) -> Result<String, NetworkError> {
        self.listener.local_addr()
            .map(|addr| addr.to_string())
            .map_err(NetworkError::Io)
    }

    /// 解析原始消息为结构化数据
    fn parse_message(&self, data: &[u8]) -> Result<ProcessMessage, NetworkError> {
        if let Ok(reg) = Registration::decode(data) {
            Ok(ProcessMessage::Registration(reg))
        } else if let Ok(hb) = Heartbeat::decode(data) {
            Ok(ProcessMessage::Heartbeat(hb))
        } else {
            log::warn!("Received unknown message type ({} bytes)", data.len());
            Err(NetworkError::Decode(prost::DecodeError::new("Unknown message type")))
        }
    }

    /// 创建新的网络实例
    pub fn new(bind_addr: &str, port: u16) -> Result<Self, NetworkError> {
        let addr = format!("{}:{}", bind_addr, port).parse().map_err(|e| {
            NetworkError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })?;
        
        let mut listener = TcpListener::bind(addr)?;
        let poll = Poll::new()?;
        
        poll.registry().register(
            &mut listener,
            Token(0),
            Interest::READABLE,
        )?;

        Ok(Self {
            listener,
            poll,
            connections: Vec::new(),
        })
    }

    /// 轮询网络事件
    /// 返回解析后的消息列表，由调用方处理业务逻辑
    pub fn poll_events(&mut self) -> Result<Vec<ProcessMessage>, NetworkError> {
        let mut events = Events::with_capacity(128);
        self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        let mut messages = Vec::new();

        for event in events.iter() {
            match event.token() {
                Token(0) => { // 新连接
                    let (mut stream, _) = self.listener.accept()?;
                    self.poll.registry().register(
                        &mut stream,
                        Token(self.connections.len() + 1),
                        Interest::READABLE,
                    )?;
                    self.connections.push(stream);
                }
                token => { // 已有连接
                    let idx = usize::from(token.0) - 1;
                    if let Some(stream) = self.connections.get_mut(idx) {
                        let mut buf = [0; 1024];
                        
                        // 非阻塞读取
                        let mut retries = 0;
                        let n = loop {
                            match stream.read(&mut buf) {
                                Ok(n) => break n,
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    retries += 1;
                                    if retries > 5 {
                                        log::warn!("Max retries reached while receiving message");
                                        break 0;
                                    }
                                    std::thread::sleep(Duration::from_millis(10));
                                    continue;
                                }
                                Err(e) => return Err(NetworkError::Io(e)),
                            }
                        };

                        if n > 0 {
                            log::debug!("Received message ({} bytes)", n);
                            messages.push(self.parse_message(&buf[..n])?);
                        }
                    }
                }
            }
        }
        Ok(messages)
    }
    /// 发送任务响应
    pub fn send_response(&mut self, client_idx: usize, response: &Response) -> Result<(), NetworkError> {
        let mut buf = Vec::new();
        response.encode(&mut buf)?;
        
        if let Some(stream) = self.connections.get_mut(client_idx) {
            // 非阻塞写入
            let mut written = 0;
            while written < buf.len() {
                match stream.write(&buf[written..]) {
                    Ok(n) => written += n,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => return Err(NetworkError::Io(e)),
                }
            }
            log::debug!("Successfully sent response ({} bytes)", buf.len());
        }
        Ok(())
    }
}

