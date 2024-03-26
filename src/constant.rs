
pub const HELP: &str = r###"
利用tokio异步机制，可在一堆远程服务器上执行linux命令，以及上传和下载单个文件。
所有连接顺序连接成功后同时执行命令

支持的命令如下：
command farmat:
    rssh_async [regular expression] exec [command...]
    rssh_async [regular expression] get remote_file local_dir
    rssh_async [regular expression] put local_file remote_file

note:
    1. RE == [regular expression]
        RE.is_match?(ip || hostname || group_name):continue;
        if RE == all then RE = ".*";

example:
    1. rssh_async exec ls perf/
    2. rssh_async all exec ls perf/
    3. rssh_async [regex] exec ls perf/
    4. rssh_async [regex] get ~/perf/res.nmon ./
    5. rssh_async [regex] put nmon_rhel7 ~/oss/bin/nmon
"###;