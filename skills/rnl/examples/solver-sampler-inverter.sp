* CMOS transmission-gate sampler followed by a CMOS inverter
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4");}
* For a solver (references/good-drawings.md): The input port, the transmission gate, the hold capacitor, the inverter and the output port from left to right, the ports level with the centres of the gate and the inverter; every net routed, the clock gates facing up and down; no CURRENT, since neither the switches nor the inverter carry a static current.
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
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*   %tg = #MP1(pmos) #MN1(nmos) @AS $TRANSMISSION_GATE;
*   %tg(switch) #CH(capacitor) @AS $SC_SAMPLER;
*   %inv = #MP2(pmos) #MN2(nmos) @AS $CMOS_INVERTER;
*   %p_in = $IO_PORT @ATTACH &in;
*   %p_out = $IO_PORT @ATTACH &out;
*   {%p_in %tg #CH %inv %p_out} @ORDER("left-to-right");
*   {%p_in %tg::position %inv::position %p_out} @ALIGN("y");
*   %p_in @ORIENT("left");
*   %p_out @ORIENT("right");
*   #MP1:g @ORIENT("up");
*   #MN1:g @ORIENT("down");
* }
.ends tgate_sampler
.end
