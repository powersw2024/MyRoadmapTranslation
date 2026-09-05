# 文件系统层级与路径

> 对应知识点：k8-1-1 · 预计 30 分钟 · 难度 ★☆☆

## 学习目标

- 说出 FHS 主要目录的用途
- 熟练使用 pwd/ls/cd 与绝对/相对路径

## 一、树与根

Linux 只有一棵树，根是 `/`。磁盘分区、U 盘都被「挂载」到树的某个节点上（`mount`）。「一切皆文件」：普通文件、目录、设备（`/dev/sda`）、进程信息（`/proc/1234/`）、内核参数（`/sys/`）都以文件形式暴露——**一套接口（open/read/write）操作一切**。

## 二、FHS 核心目录

| 目录 | 用途 |
| --- | --- |
| `/etc` | 系统与应用**配置**（nginx.conf、passwd） |
| `/home` | 普通用户的家目录（/home/alice） |
| `/root` | root 用户的家目录 |
| `/var` | 可变数据：日志（/var/log）、缓存、邮件 |
| `/tmp` | 临时文件（重启通常清空） |
| `/usr` | 安装的软件与库（/usr/bin、/usr/lib） |
| `/bin` `/sbin` | 基础命令（常为 /usr/bin 的符号链接） |
| `/dev` | 设备文件（/dev/null、/dev/sda） |
| `/proc` | 内核/进程的实时信息视图 |
| `/opt` | 第三方大型软件 |

## 三、路径操作

```bash
pwd                  # 我在哪（绝对路径）
cd /etc/nginx        # 绝对路径（以 / 开头）
cd ../logs           # 相对路径（.. 上级，. 当前，~ 家目录）
ls -la               # 列表含隐藏文件；-l 详细信息
ls /proc/self        # 「自己这个进程」的实时视图
cat /proc/cpuinfo    # 读内核数据的例子
```

## 四、动手巡礼

依次执行并记录发现：

```bash
ls /etc | head          # 配置长什么样
ls /var/log | head      # 有哪些日志
cat /proc/meminfo | head
ls -l /dev/null         # 设备文件长什么样
echo hi > /dev/null     # 黑洞：丢弃输出
```

## 验证清单

- [ ] 说出 /etc、/var、/home、/dev、/proc 的用途
- [ ] 用相对路径完成过一次移动
- [ ] 解释 /dev/null 是什么

## 延伸阅读

- Filesystem Hierarchy Standard — https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html
- man hier（层级说明的手册页）— `man hier`
