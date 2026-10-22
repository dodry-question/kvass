import sys
from PIL import Image

# crop.py src x y w h z out
src, out = sys.argv[1], sys.argv[7]
x, y, w, h, z = map(int, sys.argv[2:7])
img = Image.open(src).crop((x, y, x + w, y + h))
img = img.resize((w * z, h * z), Image.NEAREST)
img.save(out)
print("saved", out)
