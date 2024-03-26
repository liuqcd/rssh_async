mod server;
mod constant;
use std::io::Write;
use std::path::PathBuf;
use clap::{Parser, arg, command, ArgAction, Subcommand};
use serde_json;
use tokio;
use server::Server;
use server::SessionInfo;
use anyhow::Result;
use tokio::fs;
use std::io::{Error, ErrorKind};
use regex::Regex;
use tokio::net::TcpStream;
use tokio::time;
use std::time::Instant;
use std::time::Duration;
use ssh2::Session;
use tokio::sync::mpsc;
use tokio_stream::{self as stream, StreamExt};
use std::io::Read;
use log::{debug, info, warn, error};
use std::time::SystemTime;
use chrono::offset::Local;


/// 利用tokio异步机制，可在一堆远程服务器上执行linux命令，以及上传和下载单个文件。
/// 所有连接成功后执行命令
#[derive(Parser, Debug)]
#[command(author="liuqxx", version="0.1.0", long_about = constant::HELP)]
struct Args {
    /// 指定一个自定义的配置文件，默认为: server.json
    #[arg(short, long, value_name="FILE")]
    config: Option<PathBuf>,

    /// 指定一个日志输出文件(追加)，默认不输出日志到文件
    #[arg(short, long, value_name="FILE")]
    logfile: Option<PathBuf>,

    /// 开启DEBUG日志
    #[arg(short, long, action = ArgAction::SetTrue)]
    debug: bool,

    /// 打印内置配置文件（模板）信息
    #[arg(short, long, action = ArgAction::SetTrue)]
    print_config_info: bool,

    /// 正则表达式，代表要批量操作的远程服务器，表达式匹配配置文件中valid为true的信息，匹配项有:[$group.name || $group.name.hostname || $group.name.ip], 默认匹配所有: .*
    regex: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 在远程服务器上执行命令
    Exec {
        /// 在远程服务器上执行的命令，如: ls perf/
        statement: Vec<String>,
    },
    /// 从远程服务器上下载单个文件到本地目录
    Get {
        /// 要从远程服务器下载文件，只能为单个文件且不能为目录
        source_file: PathBuf,
        /// 下载文件保存在本地的目录，并重命名文件名格式为: [hostname]_[ip]_[filename]
        dest_dir: PathBuf,
    },
    /// 上传本地单个文件到远程服务器上
    Put {
        /// 要上传到远程服务器上的文件，只能为单个文件且不能为目录
        source_file: PathBuf,
        /// 上传到远程服务器上的目录
        dest_dir: PathBuf,
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    // 处理传入的程序的参数
    let args = Args::parse();

    let mut log = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::DateTime::<Local>::from(SystemTime::now()).format("%m-%m %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(std::io::stdout());

    log = if let Some(logfile) = args.logfile {
        log.chain(fern::log_file(logfile)?)
    }else { log };

    log = if args.debug {
        log.level(log::LevelFilter::Debug)
    }else {
        log.level(log::LevelFilter::Info)
    };

    log.apply()?;

    if args.print_config_info {
        let cfg: Server = Default::default();
        // info!("{}", serde_json::to_string_pretty(&cfg)?);
        info!("{}", serde_json::to_string(&cfg)?);
        return Ok(());
    }


    let path = if let Some(path) = args.config {
        path
    }else {
        PathBuf::from("server.json")
    };

    let re = if let Some(regex) = args.regex {
        match regex.as_str() {
            "all" => Regex::new(".*")?,
            _ => Regex::new(regex.as_str())?,
        }
    }else {
        Regex::new(".*")?
    };

    // 解析配置文件
    let json = fs::read_to_string(path.clone()).await
                .map_err(|e| anyhow::Error::msg(
                            format!("读取配置文件: {:?} 失败: {}", path, e)
                        ))?;
    let cfg: Server = serde_json::from_str(&json)
                .map_err(|e| anyhow::Error::msg(
                            format!("反序列化配置文件: {:?} 失败: {}", path, e)
                        ))?;
    let vec_session_info = cfg.valid_session_info()
                .ok_or(anyhow::Error::msg(
                            format!("配置文件: {:?} 没有valid为true的信息", path)
                        ))?;


    let (tx, mut rx) = mpsc::channel(10);

    match args.command {
        Some(Commands::Exec {statement}) => {
            let vec_regex = find_regex_objects(&vec_session_info, re);
            debug!("vec_regex: {:#?}", vec_regex);
            // 连接服务器
            match link(vec_regex, tx.clone()).await {
                // 执行语句
                Ok(_) => {
                    if let Err(e) = exec(&mut rx, statement).await {
                        warn!("{:?}", e);
                    };
                },
                Err(e) => error!("{:?}", e),
            }
        },
        Some(Commands::Put {source_file, dest_dir}) => {
            let vec_regex = find_regex_objects(&vec_session_info, re);
            debug!("vec_regex: {:#?}", vec_regex);
            match link(vec_regex, tx.clone()).await {
                Ok(_) => {
                    if let Err(e) = scp_put(&mut rx, source_file, dest_dir).await {
                        error!("上传文件失败: {:?}", e);
                    };
                },
                Err(e) => error!("上传文件失败: {:?}", e),
            }
        },
        Some(Commands::Get {source_file, dest_dir}) => {
            let vec_regex = find_regex_objects(&vec_session_info, re);
            debug!("vec_regex: {:#?}", vec_regex);
            match link(vec_regex, tx.clone()).await {
                Ok(_) => {
                    if let Err(e) = scp_get(&mut rx, source_file, dest_dir).await {
                        error!("下载文件失败: {:?}", e);
                    }
                },
                Err(e) => error!("下载文件失败: {:?}", e),
            }
        },
        None => {
            error!("输入参数不对，请查看帮助: rssh_async -h");
        }
    }
     
    Ok(())
}

// 连接服务器
async fn link(vec_regex: Vec<SessionInfo>, tx: mpsc::Sender::<Option<(SessionInfo, Session)>>) -> Result<()> {
    let mut handles = Vec::new();
    let mut stream = stream::iter(vec_regex);
    while let Some(info) = stream.next().await {
        let tx1 = tx.clone();
        let handle = tokio::spawn( async move {
            link_solo(info, tx1).await
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.await??; 
    }
    tx.send(None).await?;
    Ok(())
}

async fn link_solo(info: SessionInfo, tx: mpsc::Sender::<Option<(SessionInfo, Session)>>) -> Result<()> {
    let now = Instant::now();
    let tcp = time::timeout(Duration::from_secs(3), TcpStream::connect(info.to_socket_addrs())).into_inner().await
                    .map_err(|e| attach_info(e.into(), &info))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(&info.user, &info.password)
            .map_err(|e| attach_info(e.into(), &info))?;
    debug!( "{:#}, connect ok, elapsed: {}ms", info.hostname_ip(), now.elapsed().as_millis());
    tx.send(Some((info.clone(), session))).await
        .map_err(|e| attach_info(e.into(), &info))?;
    Ok(())
}

fn attach_info(e: anyhow::Error, info: &SessionInfo ) -> anyhow::Error {
    anyhow::Error::msg(
        format!("{}, {}", e, info)
    )
}

fn find_regex_objects(vec: &Vec<SessionInfo>, re: Regex) -> Vec<SessionInfo> {
    vec.iter()
        .filter(|info| re.is_match(&info.hostname) || re.is_match(&info.ip) || re.is_match(&info.groupname))
        .map(|info| info.clone())
        .collect()
}

async fn exec(rx: &mut mpsc::Receiver::<Option<(SessionInfo, Session)>>, statement: Vec<String>) -> Result<()> {

    let mut command = String::new();
    statement.iter().for_each(|s| {
        command.push_str(s);
        command.push(' ');
    });

    let mut handles = Vec::new();
    loop {
        let c = command.clone();
        if let Some(value) = rx.recv().await {
            if let Some((info, session)) = value {
                let handle = tokio::spawn(async move {
                    exec_solo(info, session, c).await
                });
                handles.push(handle);
            }else {
                break;
            }
        }
    }
    for handle in handles {
        handle.await??; 
    }
    Ok(())
}

async fn exec_solo(info: SessionInfo, session: Session, command: String) -> Result<()> {
    let now = Instant::now();
    let mut channel = session.channel_session()?;
    channel.exec(&command).map_err(|e| attach_info(e.into(), &info))?;
    let mut res: Vec<u8> = Vec::new();
    channel.read_to_end(&mut res).map_err(|e| attach_info(e.into(), &info))?;
    let mut stderr = channel.stderr();
    let _ = stderr.read_to_end(&mut res).ok();
    info!("{}, command:{}, elapsed: {}ms, res:\n{}",
        info.hostname_ip(),
        command,
        now.elapsed().as_millis(),
        String::from_utf8_lossy(res.as_slice()),
    );
    let _ = channel.wait_close();
    Ok(())
}

async fn scp_put(rx: &mut mpsc::Receiver::<Option<(SessionInfo, Session)>>, local_file: PathBuf, remote_file: PathBuf) -> Result<()> {
    let mut handles = Vec::new();
    loop {
        let lf = local_file.clone();
        let rf = remote_file.clone();
        if let Some(value) = rx.recv().await {
            if let Some((info, session)) = value {
                let handle = tokio::spawn(async move {
                    scp_send(info, session, lf, rf).await
                });
                handles.push(handle);
            }else {
                break;
            }
        }
    }
    for handle in handles {
        handle.await??;
    }
    Ok(())
}

async fn scp_send(info: SessionInfo, session: Session, local_file: PathBuf, remote_path: PathBuf) -> Result<()> {
    use std::path::Path;

    let now = Instant::now();
    let local_path = Path::new(&local_file);
    let buf = fs::read(local_path).await
                .map_err(|e| attach_info(e.into(), &info))?;

    let len = buf.len();

    let local_file_name = local_path.file_name().ok_or(Error::new(ErrorKind::Other, "从localfile中解析filename失败"))
                                    .map_err(|e| attach_info(e.into(), &info))?;

    let mut pathbuf = info.home_buf_path();
    let path = if remote_path.is_absolute() {
                remote_path.as_path()
            }else {
                pathbuf.push(remote_path);
                pathbuf.push(local_file_name);
                pathbuf.as_path()
            };

    let mut remote_file = session.scp_send(path, 0o644, len as u64, None)
                        .map_err(|e| attach_info(e.into(), &info))?;

    remote_file.write_all(&buf)
                .map_err(|e| attach_info(e.into(), &info))?;
    info!(
        "{}, local_file: {:?}, remote_path: {:?}, elapsed: {}ms, upload successfully",
        info.hostname_ip(),
        local_file,
        pathbuf,
        now.elapsed().as_millis(),
    );
    Ok(())
}

pub async fn scp_get(rx: &mut mpsc::Receiver::<Option<(SessionInfo, Session)>>, remote_file: PathBuf, local_path: PathBuf) -> Result<()> {
    let mut handles = Vec::new();
    loop {
        let rf = remote_file.clone();
        let lp = local_path.clone();
        if let Some(value) = rx.recv().await {
            if let Some((info, session)) = value {
                let handle = tokio::spawn(async move {
                    scp_recv(info, session, rf, lp).await
                });
                handles.push(handle);
            }else {
                break;
            }
        }
    }
    for handle in handles {
        handle.await??;
    }
    Ok(())
}

async fn scp_recv(info: SessionInfo, session: Session, remote_file: PathBuf, local_path: PathBuf) -> Result<()> {
    let now = Instant::now();

    let mut rpathbuf = info.home_buf_path();

    rpathbuf.push(remote_file.clone());

    let rfilename = remote_file.file_name().ok_or(Error::new(ErrorKind::Other, "从remote_path中解析filename失败"))
                                .map_err(|e| attach_info(e.into(), &info))?;

    let mut lpathbuf = local_path.clone();

    let (mut channel, _file_stat)= session.scp_recv(&rpathbuf)
        .map_err(|e| attach_info(e.into(), &info))?;

    let mut content = Vec::new();
    channel.read_to_end(&mut content)
            .map_err(|e| attach_info(e.into(), &info))?;

    if lpathbuf.is_dir() {
        lpathbuf.push(&format!(
            "{}_{}",
            info.hostname_ip(),
            rfilename.to_string_lossy()
        ));
    }
    fs::write(&lpathbuf, &content).await
            .map_err(|e| attach_info(e.into(), &info))?;
    info!(
        "{}, remote_file:{:?}, local_file:{:?}, elapsed: {}ms, download successfully",
        info.hostname_ip(),
        rpathbuf,
        lpathbuf,
        now.elapsed().as_millis()
    );
    Ok(())
}