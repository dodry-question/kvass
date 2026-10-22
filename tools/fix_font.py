# -*- coding: utf-8 -*-
"""Чинит assets/fonts/pixelify.ttf.

В шрифте Pixelify Sans из google/fonts отсутствуют глифы «О» (U+041E) и
«П» (U+041F), а «Э» нарисована копией «З». Из-за этого в игре:

  * «О» и «П» падают в .notdef (пустой квадрат) -> «П» читается как «О»
    («ВЫПОЛНЕНО» -> «ВЫООЛНЕНО»);
  * «Э» выглядит как «З» («САНЭПИДЕМНАДЗОР» -> «САНЗПИДЕМ...»).

Скрипт достраивает недостающее из контуров самого шрифта, чтобы буквы
были в точности в стиле остальных:

  * О <- копия латинской O (в кириллице она одинакова);
  * П <- контур строчной «п», подтянутый до высоты прописных (те же
    ширины штрихов, что у «Н»: стойки 101/102, перекладина 99);
  * Э <- зеркальное отражение «З» (в кириллице Э — зеркало З);
  * э <- зеркальное отражение «з».

Сначала переменный шрифт фиксируется в статический (wght=400, тот же
инстанс, что берёт fontdue по умолчанию), чтобы правка глифов была
простой и предсказуемой.

Запуск:  python tools/fix_font.py [вход] [выход]
"""
import copy
import os
import sys

from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

SRC = sys.argv[1] if len(sys.argv) > 1 else "assets/fonts/pixelify.ttf"
DST = sys.argv[2] if len(sys.argv) > 2 else "assets/fonts/pixelify.ttf"

# Кодовые точки, которые достраиваем
U_O, U_P, U_E, U_e = 0x041E, 0x041F, 0x042D, 0x044D


def main():
    f = TTFont(SRC)
    if "fvar" in f:
        f = instancer.instantiateVariableFont(f, {"wght": 400}, updateFontNames=False)
    glyf = f["glyf"]
    cm = f.getBestCmap()
    hmtx = f["hmtx"]

    added, fixed = [], []

    # --- О: латинская O одинакова по начертанию ---
    if U_O not in cm:
        glyf["uni041E"] = copy.deepcopy(glyf["O"])
        hmtx["uni041E"] = hmtx["O"]
        added.append("О")

    # --- П: строчная «п», подтянутая до прописных ---
    # контур берём по структуре Н (стойки + перекладина сверху)
    if U_P not in cm:
        pen = TTGlyphPen(glyf)
        pen.moveTo((60, -12))
        pen.lineTo((60, 631))
        pen.lineTo((525, 631))
        pen.lineTo((525, -12))
        pen.lineTo((423, -12))
        pen.lineTo((423, 532))
        pen.lineTo((161, 532))
        pen.lineTo((161, -12))
        pen.closePath()
        glyf["uni041F"] = pen.glyph()
        hmtx["uni041F"] = hmtx["uni041D"]  # ширина как у Н
        added.append("П")

    # --- Э/э: зеркало З/з. Зеркалье меняет обход контура, поэтому
    #     после отражения порядок точек разворачиваем обратно. ---
    def contours_of(gname):
        gl = copy.deepcopy(glyf[gname])
        gl.expand(glyf)
        coords, endPts, _flags = gl.getCoordinates(glyf)
        res, start = [], 0
        for e in endPts:
            res.append([(int(p[0]), int(p[1]))
                        for p in coords[start:e + 1]])
            start = e + 1
        return res

    def mirror(src, dst, name):
        gl = copy.deepcopy(glyf[src])
        gl.expand(glyf)
        coords, endPts, _flags = gl.getCoordinates(glyf)
        xs = [p[0] for p in coords]
        mid = (min(xs) + max(xs)) / 2.0
        flipped = []
        start = 0
        for e in endPts:
            cont = [(int(2 * mid - coords[i][0]), int(coords[i][1]))
                    for i in range(start, e + 1)]
            flipped.extend(reversed(cont))
            start = e + 1
        new = TTGlyphPen(glyf)
        idx = 0
        for e in endPts:
            n = e + 1 - idx
            new.moveTo(flipped[idx])
            for k in range(1, n):
                new.lineTo(flipped[idx + k])
            new.closePath()
            idx += n
        glyf[dst] = new.glyph()
        hmtx[dst] = hmtx[src]
        fixed.append(name)

    # Проверяем именно контуры: в шрифте Э нарисована копией З,
    # хотя это разные глифы с разными именами.
    if U_E in cm and contours_of("uni042D") == contours_of("uni0417"):
        mirror("uni0417", "uni042D", "Э")
    if U_e in cm and contours_of("uni044D") == contours_of("uni0437"):
        mirror("uni0437", "uni044D", "э")

    # --- cmap: добавляем недостающие кодовые точки в обе подтаблицы ---
    cmap = f["cmap"]
    for sub in cmap.tables:
        if U_O not in sub.cmap:
            sub.cmap[U_O] = "uni041E"
        if U_P not in sub.cmap:
            sub.cmap[U_P] = "uni041F"

    os.makedirs(os.path.dirname(DST) or ".", exist_ok=True)
    f.save(DST)
    print(f"шрифт сохранён: {DST}")
    print(f"  добавлены буквы: {''.join(added) or 'нет'}")
    print(f"  исправлены буквы: {''.join(fixed) or 'нет'}")


if __name__ == "__main__":
    main()
