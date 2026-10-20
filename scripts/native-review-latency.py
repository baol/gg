"""Measure X11 key-to-visible-selection latency on the two-file stress fixture."""
import sys
import ctypes as C
import os, subprocess, time
from pathlib import Path
X = C.CDLL('libX11.so.6')
T = C.CDLL('libXtst.so.6')
D = C.c_void_p
W = C.c_ulong
I = C.c_int
U = C.c_uint
X.XOpenDisplay.argtypes = [C.c_char_p]
X.XOpenDisplay.restype = D
X.XDefaultRootWindow.argtypes = [D]
X.XDefaultRootWindow.restype = W
X.XQueryTree.argtypes = [D, W, C.POINTER(W), C.POINTER(W), C.POINTER(C.POINTER(W)), C.POINTER(U)]
X.XFetchName.argtypes = [D, W, C.POINTER(C.c_char_p)]
X.XFree.argtypes = [C.c_void_p]
X.XSetInputFocus.argtypes = [D, W, I, W]
X.XMapWindow.argtypes = [D, W]
X.XSync.argtypes = [D, I]
X.XFlush.argtypes = [D]
X.XStringToKeysym.argtypes = [C.c_char_p]
X.XStringToKeysym.restype = W
X.XKeysymToKeycode.argtypes = [D, W]
X.XKeysymToKeycode.restype = U
T.XTestFakeMotionEvent.argtypes = [D, I, I, I, W]
T.XTestFakeButtonEvent.argtypes = [D, U, I, W]
T.XTestFakeKeyEvent.argtypes = [D, U, I, W]
X.XGetImage.argtypes = [D, W, I, I, U, U, W, I]
X.XGetImage.restype = D
X.XGetPixel.argtypes = [D, I, I]
X.XGetPixel.restype = W
X.XDestroyImage.argtypes = [D]
d = X.XOpenDisplay(None)
assert d
root_window = X.XDefaultRootWindow(d)

def pixel(x, y):
    image = X.XGetImage(d, root_window, x, y, 1, 1, W(-1), 2)
    assert image
    value = X.XGetPixel(image, 0, 0)
    X.XDestroyImage(image)
    return (value >> 16 & 255, value >> 8 & 255, value & 255)
app = subprocess.Popen([str(Path(sys.argv[1]).resolve()), str(Path(sys.argv[2]).resolve())], env={**os.environ, 'GG_OFFLINE': '1'})

def move(x, y):
    T.XTestFakeMotionEvent(d, -1, x, y, 0)
    X.XFlush(d)

def click(x, y):
    move(x, y)
    T.XTestFakeButtonEvent(d, 1, 1, 0)
    X.XFlush(d)
    time.sleep(0.1)
    T.XTestFakeButtonEvent(d, 1, 0, 0)
    X.XFlush(d)
try:
    window = None
    for _ in range(100):
        root = W()
        parent = W()
        children = C.POINTER(W)()
        count = U()
        X.XQueryTree(d, X.XDefaultRootWindow(d), C.byref(root), C.byref(parent), C.byref(children), C.byref(count))
        for j in range(count.value):
            name = C.c_char_p()
            if X.XFetchName(d, children[j], C.byref(name)) and name.value:
                if 'good git' in name.value.decode(errors='replace'):
                    window = children[j]
                X.XFree(name)
        if children:
            X.XFree(children)
        if window:
            break
        time.sleep(0.1)
    assert window
    X.XMapWindow(d, window)
    X.XSync(d, 0)
    time.sleep(1)
    X.XSetInputFocus(d, window, 2, 0)
    X.XFlush(d)
    time.sleep(1)
    click(976, 20)
    move(1100, 20)
    time.sleep(1)
    ys = [y for y in range(100, 400) if (lambda c: c[1] > c[0] + 20 and c[2] > c[0] + 20)(pixel(20, y))]
    assert ys, 'Selected row not found; use the default 1120x740 window on an isolated X11 display'
    center = (min(ys) + max(ys)) // 2
    samples = []
    for index in range(80):
        down = index % 2 == 0
        code = X.XKeysymToKeycode(d, X.XStringToKeysym(b'Down' if down else b'Up'))
        start = time.perf_counter()
        T.XTestFakeKeyEvent(d, code, 1, 0)
        T.XTestFakeKeyEvent(d, code, 0, 0)
        X.XFlush(d)
        target = center + 26 if down else center
        while True:
            if (lambda c: c[1] - c[0] > 15 and c[2] - c[0] > 25)(pixel(20, target)):
                break
            if time.perf_counter() - start >= 2:
                raise AssertionError(f'Selection did not render: iteration={index}, target={target}')
            time.sleep(0.001)
        samples.append((time.perf_counter() - start) * 1000)
        time.sleep(0.1)
    samples.sort()
    print(f'Native key-to-visible-selection: p50={samples[40]:.2f}ms p95={samples[76]:.2f}ms max={samples[-1]:.2f}ms; samples=80', flush=True)
    assert samples[76] < 50, samples
finally:
    app.terminate()
    try:
        app.wait(timeout=5)
    except subprocess.TimeoutExpired:
        app.kill()
        app.wait()
