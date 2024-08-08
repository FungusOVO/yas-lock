## 介绍

基于[yas](https://github.com/wormtql/yas)，开发的加解锁功能。

## 使用

用于加解锁的 lock.json 文件需与 lock_artifact.exe、lock_relic.exe 放在同一目录下

原神圣遗物加解锁

```
lock_artifact.exe
```

星铁遗器加解锁

```
lock_relic.exe
```

其他命令行参数（建议管理员运行）：

```shell
lock_artifact.exe --help
lock_relic.exe --help
```

鼠标移动后点击间隔，加解锁没有锁定/解锁成功时，建议增加间隔时间

```shell
lock_artifact.exe --click-time <time>
lock_relic.exe --click-time <time>
```

圣遗物/遗器加解锁后的等待时间，锁定/解锁所在行发生错误时，建议增加间隔时间

```shell
lock_artifact.exe --select-time <time>
lock_relic.exe --select-time <time>
```

### 注意

- 打开原神/星铁，并切换到背包页面，将背包拉到最上面
- 不是所有窗口比例都支持，原神推荐 16:9 的分辨率（如 1600x900, 1920x1080, 3840x2160），星铁为必须 16:9 的分辨率
- 加解锁过程中不要对鼠标做任何操作
- 加解锁过程中，鼠标右键终止
- 当前仅支持中文环境，若默认系统为非中文，请前往游戏设置界面修改 Language 为“简体中文”，否则无法读取原神窗口
- 当前仅支持键鼠作为控制设备，暂不支持手柄。
