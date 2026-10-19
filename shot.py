import sys
from PIL import ImageGrab

out = sys.argv[1] if len(sys.argv) > 1 else "selftest_out/z_live.png"
img = ImageGrab.grab()
img.save(out)
print(f"saved {out} ({img.width}x{img.height})")
