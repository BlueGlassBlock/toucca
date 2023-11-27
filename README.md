# Toucca

> Touch support for WACCA(SDFE).

English | [简体中文](README.zh.md)

## Configuration

Configuration is done via `segatools.ini`.

```ini
[touch]
divisions = 8 # Number of divisions to split the window radius into.
pointer_radius = 1 # Radius of cells triggered by a pointer
mode = 0 # 0 for absolute mode, 1 for relative mode
```

### Absolute mode config

Ring 0 is the most inner ring, and ring 3 is the most outer ring.

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

If the ring value is not supplied or set to -1, will automatically determine from `divisions` (making ring3 match the *actual* most outer ring, and layout other rings accordingly).

### Relative mode config

```ini
[touch]
relative_start = 1 # Start of relative ring
relative_threshold = 1 # Physical ring required to cross to change mapped ring
```

## Details

### Pointer Radius

`touch.pointer_radius` controls how many cells are triggered by a pointer.

This only affects width of the pointer, not height.

For example, if `touch.pointer_radius` is set to `1`, the pointer will be 1 cells wide, 3 cells wide if set to `2`.

### Touch Radius Compensation

By default, the radius of the play area is determined by window width, which may cause weirdness when the play area is not completely displayed.

By default, an extra 30px is added to the radius to compensate for this.

You can change compensation radius with config `touch.radius_compensation`, which could be positive or negative.

## License

[GPL 3.0 or later](LICENSE)