* Manifest fixture: an entry with a relative include, a bare include found in the pool, and two packages
* @RNL(type="header") { @VERSION("0.3"); @USE-PACKAGE(name="demo-local", version="1.0"); @USE-PACKAGE(name="demo-remote", version="2.0") }
.include "cells/amp.sp"
.include models.inc
V1 in 0 1
X1 in out amp
R9 out 0 10k
* @RNL { #V1 @PLACE(0, 0); #X1 @PLACE(10, 0); #R9 @PLACE(20, 0) }
.end
