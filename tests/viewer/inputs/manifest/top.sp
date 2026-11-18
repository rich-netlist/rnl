* Manifest fixture: an entry with a relative include, a bare include found in the pool, and two packages
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4"); @USE-PACKAGE(name="demo-local", version="1.0"); @USE-PACKAGE(name="demo-remote", version="2.0") }
* @RNL { /* rnl upgrade wrote this record: it keeps the labels and ports the file had before 0.4 */ # @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET; $SUPPLY_NET_SET @REMOVE-FROM $PORT_FILL_SET; $GROUND_NET_SET @REMOVE-FROM $PORT_FILL_SET }
.include "cells/amp.sp"
.include models.inc
V1 in 0 1
X1 in out amp
R9 out 0 10k
* @RNL { #V1 @PLACE(0, 0); #X1 @PLACE(10, 0); #R9 @PLACE(20, 0) }
.end
