use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::net::SocketAddr;

/// 代表整个json配置文件
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    // #[serde(flatten)]
    groups: Vec<Group>,
}

impl Server {
    pub fn valid_session_info(&self) -> Option<Vec<SessionInfo>> {
        let mut vec = Vec::new();
        self.groups.iter()
            .filter(|g| g.valid())
            .for_each(|g| if let Some(mut s) = g.valid_session_info() { vec.append(&mut s); });
        if vec.is_empty() {
            None
        } else {
            Some(vec)
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            groups: vec![Default::default()]
        }
    }
}


/// 多行连接信息可组成一组
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Group {
    name: String,
    members: Vec<Member>,
    valid: bool,
}

impl Group {
    pub fn valid_session_info(&self) -> Option<Vec<SessionInfo>> {
        if self.valid {
            let mut vec = Vec::new();
            self.members
                .iter()
                .filter(|a| a.valid())
                .for_each(|a| { 
                    let m = SessionInfo {
                        hostname: a.hostname.clone(),
                        ip: a.ip.clone(),
                        port: a.port,
                        user: a.user.clone(),
                        password: a.password.clone(),
                        groupname: self.name.clone(),
                    };
                    vec.push(m);
                });

            Some(vec)
        } else {
            None
        }
    }
    pub fn valid(&self) -> bool {
        self.valid
    }
}

impl Default for Group {
    fn default() -> Self {
        Self {
            name: Default::default(),
            members: vec![Default::default()],
            valid: true,
        }
    }
}

/// 每一行连接信息即一个Info实例。
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Member {
    pub hostname: String,
    pub ip: String,
    pub port: usize,
    pub user: String,
    pub password: String,
    pub valid: bool,
}

impl Member {
    // pub fn addr(&self) -> SocketAddr {
    //     use std::str::FromStr;
    //     let s = format!("{}:{}", self.ip, self.port);
    //     SocketAddr::from_str(s.as_str()).unwrap()
    // }

    pub fn valid(&self) -> bool {
        self.valid
    }
}

impl Default for Member {
    fn default() -> Self {
        Self {
            hostname: Default::default(),
            ip: Default::default(),
            port: 22,
            user: String::from("cx"),
            password: String::from("chaxun"),
            valid: true,
        }
    }
}

impl fmt::Display for Member {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "hostname:{}, ip:{}, port:{}, user:{}, password:{}, valid:{}",
            self.hostname, self.ip, self.port, self.user, self.password, self.valid
        )
    }
}

/// 每一行连接信息即一个Info实例。
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SessionInfo {
    pub hostname: String,
    pub ip: String,
    pub port: usize,
    pub user: String,
    pub password: String,
    pub groupname: String,
}

impl SessionInfo {
    pub fn home_buf_path(&self) -> PathBuf {
        let mut path = PathBuf::from("/");
        if self.user == *"root" {
            path.push("root");
        } else {
            path.push("home");
            path.push(&self.user);
        }
        path
    }

    pub fn hostname_ip(&self) -> String {
        format!("{}_{}", self.hostname, self.ip)
    }
    pub fn to_socket_addrs(&self) -> SocketAddr {
        use std::str::FromStr;
        let s = format!("{}:{}", self.ip, self.port);
        SocketAddr::from_str(s.as_str()).expect(&format!("{}, 转换为SocketAddr失败", s))
    }
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, port:{}, user:{}, password:{}",
            self.hostname_ip(),
            self.port,
            self.user,
            self.password,
        )
    }
}
