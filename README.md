# Toucca (WPF)

> Any2WACCAi's WPF wrapper, adding touch support for WACCA(SDFE).

## Setup

Toucca controls the game via serial port.

You need to bind COM3 and COM5, COM4 and COM6 using [com0com](https://sourceforge.net/projects/com0com/), enabling buffer overrun at the meantime.

Read more about configuration at the [WACVR](https://github.com/xiaopeng12138/WACVR#serial-not-recommended) project.

You may need to set `touch.enable` to `0` to disable the hook-based touch input.

After setting up serial port pairs, launch ``toucca.exe` before launching WACCA and you are good to go,
toucca will automatically connect to the game & resize its overlay.

## Customization

Toucca used WebView2 to serve the content in `web` folder, so you can customize the overlay by editing the files there
(most of the time you'll find `controller.js` useful).

## Credits

- Any2WACCAi by Raymonf
- Any2WACCA_with_WACCAVCon by Mishe.W#7250
- WACVR by xiaopeng12138
- MaiTouchSensorEmulator by Leapward-Koex

## License

[GPL 3.0 or later](LICENSE)