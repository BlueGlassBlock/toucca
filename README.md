# Toucca

> Touch support for WACCA.

## Configuration

Configuration is done via `segatools.ini`.

```ini
[touch]
divisions = 8 # Number of divisions to split the window radius into.
mode = 0 # 0 for absolute mode, 1 for relative mode (not implemented)
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
```

If the ring value is not supplied or set to -1, will automatically determine from `divisions` (making ring3 match the *actual* most outer ring, and layout other rings accordingly).

### Relative mode config

```ini
[touch]
relative_start = 1 # Start of relative ring
relative_threshold = 1 # Physical ring required to cross to change mapped ring
```

## Details

### Touch Radius Compensation

By default, the radius of the play area is determined by window width, which may cause weirdness when the play area is not completely displayed.

By default, an extra 30px is added to the radius to compensate for this.

You can change compensation radius with config `touch.radius_compensation`, which could be positive or negative.

## License

[GPL 3.0 or later](LICENSE)