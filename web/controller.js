const el = document.getElementById('canvas');
const ctx = document.getElementById('canvas').getContext('2d');

const hostname = window.location.hostname.length > 0 ? window.location.hostname : '127.0.0.1';
const socket = new WebSocket('ws://' + hostname + ':25730');
el.width = document.body.clientWidth;
el.height = document.body.clientHeight;
let w = el.width;
let h = el.height;
let center_x = w / 2;
let center_y = h / 2;
let base_radius = Math.min(center_x, center_y)

let RADIUS_LIST = [0.6, 0.7, 0.8, 0.9, 1.0]
let FILL_COLOR = 'rgb(0, 0, 255, 30%)'
let FRAME_COLOR = 'rgb(255, 255, 255, 2%)'

let current_touches = []
for (let i = 0; i < 240; ++i) {
    current_touches.push(false);
}

function uint8ArrayToBase64(uint8Array) {
    return btoa(String.fromCharCode(...uint8Array));
}

function buf2hex(buffer) { // buffer is an ArrayBuffer
  return [...new Uint8Array(buffer)]
      .map(x => x.toString(16).padStart(2, '0'))
      .join('');
}

function send() {
    const buffer = new Int8Array(30);
    for (let i = 0; i < 30; ++i) {
        buffer[i] = 0;
        for (let j = 0; j < 8; ++j) {
            if (current_touches[i * 8 + j] === true) {
                // console.log(i * 8 + j);
                buffer[i] |= 1 << j;
            }
        }
    }

    socket.send(buffer);
}

function draw() {
    ctx.fillStyle = FILL_COLOR;
    ctx.clearRect(0, 0, w, h);
    ctx.lineWidth = 1;
    ctx.strokeStyle = FRAME_COLOR;
    // Draw borders.
    for (let i of RADIUS_LIST) {
        ctx.beginPath();
        ctx.arc(center_x, center_y, base_radius * RADIUS_LIST[i], 0, 2 * Math.PI);
        ctx.closePath();
        ctx.stroke();
    }
    // 2 sides.
    for (let i = 0; i < 2; ++i) {
        // 4 rings.
        for (let j = 0; j < 4; ++j) {
            // 30 keys.
            if (i == 0) {
                for (let k = 0; k < 30; ++k) {
                    ctx.beginPath();
                    ctx.arc(center_x, center_y, base_radius * RADIUS_LIST[j], -Math.PI / 2 + Math.PI / 30 * k, -Math.PI / 2 + Math.PI / 30 * (k + 1));
                    ctx.arc(center_x, center_y, base_radius * RADIUS_LIST[j + 1], -Math.PI / 2 + Math.PI / 30 * (k + 1), -Math.PI / 2 + Math.PI / 30 * k, true);
                    ctx.closePath();
                    if (current_touches[i * 120 + j * 30 + k]) {
                        ctx.fill();
                    } else {
                        ctx.stroke();
                    }
                }
            } else {
                for (let k = 0; k < 30; ++k) {
                    ctx.beginPath();
                    ctx.arc(center_x, center_y, base_radius * RADIUS_LIST[j], 3 * Math.PI / 2 - Math.PI / 30 * k, 3 * Math.PI / 2 - Math.PI / 30 * (k + 1), true);
                    ctx.arc(center_x, center_y, base_radius * RADIUS_LIST[j + 1], 3 * Math.PI / 2 - Math.PI / 30 * (k + 1), 3 * Math.PI / 2 - Math.PI / 30 * k);
                    ctx.closePath();
                    if (current_touches[i * 120 + j * 30 + k]) {
                        ctx.fill();
                    } else {
                        ctx.stroke();
                    }
                }
            }
        }
    }
}

function update_touch(new_touches, touch) {
    let nth = 0
    // We can use isPointInPath, but it's not cool and may be slow.
    let dist = Math.sqrt((touch.clientX - center_x) * (touch.clientX - center_x) + (touch.clientY - center_y) * (touch.clientY - center_y));
    //if (dist < radius_inner2 || dist > radius) {
    //    continue;
    //}
    if (touch.clientX === center_x) {
        // Skip the border.
        return;
    }
    let rad = Math.atan((touch.clientY - center_y) / (touch.clientX - center_x));
    if (touch.clientX < center_x) {
        // Begin from the left side.
        nth += 120;
        rad = -rad;
    }
    for (let i = 1; i < 4; ++i) {
        if (dist > base_radius * RADIUS_LIST[i]) {
            nth += 30;
        } else {
            break;
        }
    }
    nth += Math.floor((rad + Math.PI / 2) / (Math.PI / 30));
    new_touches[nth] = true;
}

function update_touches(event) {
    let new_touches = []
    for (let i = 0; i < 240; ++i) {
        new_touches.push(false);
    }
    for (let i = 0; i < event.touches.length; ++i) {
        let touch = event.touches[i]
        update_touch(new_touches, touch);
        // console.log(new_touches);
    }
    for (let i = 0; i < 240; ++i) {
        // Only update when state is changed.
        if (new_touches[i] !== current_touches[i]) {
            current_touches = new_touches;
            send();
            draw();
            return;
        }
    }
}

function update_pointer(event) {
    let new_touches = []
    for (let i = 0; i < 240; ++i) {
        new_touches.push(false);
    }
    update_touch(new_touches, event);
    for (let i = 0; i < 240; ++i) {
        // Only update when state is changed.
        if (new_touches[i] !== current_touches[i]) {
            current_touches = new_touches;
            send();
            draw();
            return;
        }
    }
}

socket.addEventListener('open', function (event) {
    socket.send('G');
});
socket.addEventListener('message', function (event) {
    //console.log('Message from server ', event.data);
});


el.addEventListener("touchstart", function handleStart(evt) {
    evt.preventDefault();
    update_touches(evt);
}, false);
el.addEventListener("touchmove", function handleStart(evt) {
    evt.preventDefault();
    update_touches(evt);
}, false);
el.addEventListener("touchend", function handleStart(evt) {
    evt.preventDefault();
    update_touches(evt);
}, false);

draw();
