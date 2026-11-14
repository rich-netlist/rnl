# Bounded routing in 0.4

Each route-tree entry is routed separately, even when several trees belong to the same electrical net. The setting `join_trees=true` routes the trees of each net as one instead: the first tree of the net takes the pins and wires of the others, with its own style, so that a net that several structures wire is drawn connected rather than in pieces joined by its name. It is off by default, a file may give it in SOLVER-CONFIG, and the record replays it. The router can reorder blocked trees and try them again. It prefers paths around symbols and text, treating text as a soft obstacle and a lead along its own symbol as a last resort.

If the search exhausts its budget or cannot find a path, the remaining parts are connected by a direct orthogonal path. The solve report counts these forced connections; the ordinary geometry scores still count any crossed symbols or other obstacles. Exhausting the search budget therefore does not leave a requested tree disconnected.

The following comparison uses a zero search budget. Before, the two symbols were left without a wire. After, the requested connection is present and reported as forced. It crosses a symbol because the search had no budget to find a path around it. The symbol positions are unchanged; the drawing bounds include the added wire.

| Before | After |
| --- | --- |
| ![Unconnected after exhausting the routing budget](images/bounded-routing-before.png) | ![Forced connection after exhausting the routing budget](images/bounded-routing-after.png) |

Junctions follow electrical pin identity, not just coordinates: a pin's `position` point does not connect to the pin. Dots are written only through the configured junction definition. Released input versions retain their existing routing behavior until the language version switch.
