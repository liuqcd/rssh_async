利用tokio异步机制，可在一堆远程服务器上执行linux命令，以及上传和下载单个文件。
支持的命令如下：
command farmat:
    rssh_async [regular expression] exec [command...]
    rssh_async [regular expression] get remote_file local_dir
    rssh_async [regular expression] put local_file remote_file

example:
    1. rssh_async exec ls perf/
    2. rssh_async all exec ls perf/
    3. rssh_async [regex] exec ls perf/
    4. rssh_async [regex] get ~/perf/res.nmon ./
    5. rssh_async [regex] put nmon_rhel7 ~/oss/bin/nmon

Usage: rssh_async [OPTIONS] [REGEX] [COMMAND]

Commands:
  exec  在远程服务器上执行命令
  get   从远程服务器上下载单个文件到本地目录
  put   上传本地单个文件到远程服务器上
  help  Print this message or the help of the given subcommand(s)

Arguments:
  [REGEX]
          正则表达式，代表要批量操作的远程服务器，表达式匹配配置文件中valid为true的信息，匹配项有:[$group.name || $group.name.hostname || $group.name.ip], 默认匹配所有: .*

Options:
  -c, --config <FILE>
          指定一个自定义的配置文件，默认为: server.json

  -l, --logfile <FILE>
          指定一个日志输出文件(追加)，默认不输出日志到文件

  -d, --debug
          开启DEBUG日志

  -p, --print-config-info
          打印内置配置文件（模板）信息

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version