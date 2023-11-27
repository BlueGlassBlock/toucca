# Toucca

> Touch support for WACCA(SDFE).

[English](README.md) | 简体中文

## 配置

配置文件为 `segatools.ini`。

```ini
[touch]
divisions = 8 # 将窗口半径分割为几个区域
pointer_radius = 1 # 指针的半径
mode = 0 # 0 为绝对模式，1 为相对模式
```

## 绝对模式配置

0 环为最内环，3 环为最外环。

```ini
[touch]
ring0 = 4
ring0_start = 4
ring0_end = 4
ring1 = 5
ring1_start = 5
ring1_end = 5
ring2 = 6
ring2_start = 6
ring2_end = 6
ring3 = 7
ring3_start = 7
ring3_end = 7
```

如果环的值未设置或设置为 -1，将会自动从 `divisions` 计算（使 3 环匹配 *实际* 最外环，其他环的布局也会相应地调整）。

## 相对模式配置

```ini
[touch]
relative_start = 1 # 相对环的起始环
relative_threshold = 1 # 需要跨越的物理环数才能切换到下一个相对环
```

## 实现细节

### 指针半径

`touch.pointer_radius` 控制指针的宽度。

这只影响指针的宽度，不影响高度。

例如，如果 `touch.pointer_radius` 设置为 `1`，指针的宽度为 1 个 cell，如果设置为 `2`，宽度为 3 个 cell。

### 触摸半径补偿

默认情况下，游玩区域的半径由窗口宽度决定，这可能会导致在游玩区域没有完全显示时出现奇怪的情况。

默认情况下，额外的 30px 被添加到半径中以补偿这一点。

你可以通过 `touch.radius_compensation` 来调整这个值。

## 许可协议

[GPL 3.0 or later](LICENSE)