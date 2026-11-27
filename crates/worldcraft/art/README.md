# Worldcraft field art

The source sheets are original ImageGen artwork made for Worldcraft. They
contain no third-party scans. The terrain sheet is ordered left to right
from forest litter, granite, meadow turf and sand to snow, gravel, basalt
and exposed soil. The other two sheets supply oak, spruce, birch, meadow
grass, palm, acacia and bark.

Run `python tools/bake_field_art.py` from the repository root using the
material preparation dependencies. The script crops the sheets, removes the
magenta backing from plant cutouts, and generates seamless albedo, estimated
surface height, normals and roughness. The `.wct` files contain individually
compressed BC3 mips. The Rust client embeds the finished pack.
