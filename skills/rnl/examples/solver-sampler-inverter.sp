* CMOS transmission-gate sampler followed by a CMOS inverter
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): Switches turned sideways by one pin direction; no CURRENT, since neither the switches nor the inverter carry a static current.
.model nch nmos level=1
.model pch pmos level=1
.subckt tgate_sampler in out clk clkb vdd vss
MN1 in clk samp vss nch W=4u L=1u
MP1 in clkb samp vdd pch W=8u L=1u
CH samp vss 500f
MP2 out samp vdd vdd pch W=8u L=1u
MN2 out samp vss vss nch W=4u L=1u
* @RNL {
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   &vss @ADD-TO $GROUND_NET_SET;
*   %tg = #MP1(pmos) #MN1(nmos) @AS $TRANSMISSION_GATE;
*   %tg(switch) #CH(capacitor) @AS $SC_SAMPLER;
*   %inv = #MP2(pmos) #MN2(nmos) @AS $CMOS_INVERTER;
*   {#MP1::d #MN1::d} @ORIENT("left");
*   %pi = $IO_PORT @ATTACH &in;
*   %pi::pin @ORIENT("right");
*   %po = $IO_PORT @ATTACH &out;
*   %po::pin @ORIENT("left");
*   {%pi::pin #MN1 #CH #MN2 %po::pin} @ORDER("left-to-right");
*   {%pi::pin #MN1::d} @ALIGN("y");
*   {&in &samp &out} @ROUTE-NET;
* }
.ends tgate_sampler
.end
