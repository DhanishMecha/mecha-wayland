# Renderer

GPU compositor for the UI. Turns a per-frame list of draw primitives into pixels
in a surface, in the correct front-to-back order, on GLES 2.0 (Vivante GC7000).

## Language

**Paint order**:
The back-to-front order in which primitives are composited. The renderer's job is
to honour it across *all* primitive types, not just within one type.
_Avoid_: draw order, render order.

**z**:
A primitive's place in the global paint order. Higher z = nearer the viewer =
painted later (on top). Equal z falls back to submission order. Derived from the
UI tree (deeper nodes get a higher z, one `Z_STEP` per level) by the UI render
walk — see the UI context's `z` term and ADR 0011.
_Avoid_: depth (reserved for the GL depth buffer), layer index.

**Opaque pass**:
The first pass of a frame. Draws everything fully opaque — solid rects, opaque
quad interiors, and opaque sprites — with depth-write **on**, blending **off**,
front-to-back. Populates the depth buffer that gives the translucent pass its
early-z rejection.

**Translucent pass**:
The second pass. Draws everything that must blend — rounded/bordered quads, their
anti-aliased edges, and translucent sprites/text — with depth-**test** on
(early-z), depth-write off, blending on, globally z-sorted back-to-front. This is
where cross-type paint order is made correct.

**Opaque sprite**:
A sprite/glyph whose whole box sits over one known solid colour, so its coverage
is composited into RGB against that colour (`rgb = mix(bg, fg, mask)`) and its
alpha forced to 1. Being opaque, it joins the opaque pass and writes depth. Taken
iff `is_opaque && background.a >= 1.0`; otherwise the sprite is translucent. The
UI supplies both — see its [Inferred background] and [is_opaque] terms (ADR 0011).
_Avoid_: solid text, baked text.

**Quad-interior split**:
A quad's axis-aligned interior is emitted as a `DrawRect` (at `z + ε`) into the
opaque pass so it writes depth, while the quad's rounded edge and border draw in
the translucent pass. The interior goes opaque iff `is_opaque &&
background.over(color).a >= 1.0`, painted in `background.over(color)` — so a
*translucent* quad over a known-opaque background flattens to opaque too (collapses
to `color` when the colour is already opaque). `ε` is a sub-pixel tie-breaker
internal to one quad; it must stay far smaller than the spacing between z-layers so
it never re-orders content.

**Early-z**:
The GPU discarding translucent fragments that fall behind opaque geometry already
in the depth buffer, before shading them. The reason the opaque pass runs first.

**Texture weight**:
A per-primitive flag (`aTexWeight`, or the opaque pass's `uTexWeight` uniform)
that unifies textured and untextured primitives in one shader: the mask is
`mix(1.0, atlas.r, weight)`. Quads/rects carry weight 0 (mask = 1, the atlas
sample discarded), sprites carry 1. This lets a quad and a glyph share one bound
texture and one draw call without baking a white texel into the atlas.
_Avoid_: white texel (the alternative we didn't take).

**Atlas**:
The single R8 texture holding all glyphs, sprite images, and the white texel. One
atlas ⇒ one texture bind ⇒ one draw call per pass. Multiple atlases add
texture-bind splits but never a program switch.
