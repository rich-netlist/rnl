* Fully differential flip-around sample-and-hold with bottom-plate sampling
* In the sampling phase each input is stored on its capacitor, the bottom-plate switch opening first; in the hold
* phase the capacitors flip around to the outputs of the ideal fully differential amplifier fdamp. The two halves
* are mirror images about a horizontal axis.
* @RNL(type="header") { @USE-PACKAGE(name="rnl-std-all", version="0.4"); @VERSION("0.4") }
.model nch nmos level=1 vto=0.5 kp=200u lambda=0.05

* An ideal fully differential amplifier: outp - outn = 1e4 (inp - inn), output common mode 0.9 V
.subckt fdamp inp inn outp outn
EP outp cm inp inn 5000
EN outn cm inn inp 5000
VOCM cm 0 DC 0.9
* @RNL {
*   $IO_PORT @ATTACH &inp => %p_inp;
*   $IO_PORT @ATTACH &inn => %p_inn;
*   $IO_PORT @ATTACH &outp => %p_outp;
*   $IO_PORT @ATTACH &outn => %p_outn;
*   %routed_nets = # @SELECT-NET;
*   {$GROUND_NET_SET $SUPPLY_NET_SET $BIAS_NET_SET} @REMOVE-FROM %routed_nets;
*   %routed_nets @ROUTE-NET;
*
*   /* what the circuit is: the fully differential amplifier, its common-mode reference VOCM, and the two signal ports */
*   %bias_cm = {#VOCM} @AS $BIAS_CIRCUIT;
*   %flow_amp = {#EP #EN}(path) {&inp &inn}(source) {&outp &outn}(load) @AS $SIGNAL_FLOW;
*
*   /* the two halves are mirror images about a horizontal axis, EP above and EN below, the common-mode source on the
*      axis to the right */
*   %half_mirror = #EP(top) #EN(bottom) #VOCM(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %p_in = %p_inp(top) %p_inn(bottom) #VOCM(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %p_out = %p_outp(top) %p_outn(bottom) #VOCM(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*
*   /* the input ports on the left, then the amplifier, the common-mode source, the output ports on the right */
*   %port_order = {%p_in %half_mirror #VOCM %p_out} @ORDER("left-to-right");
*
*   /* the common-mode source's pin 1 faces left, to the amplifier */
*   %vocm_pin_orient = #VOCM:1 @ORIENT("left");
* }
.ends fdamp

VCM vcm 0 DC 0.9
VIP inp vcm SIN(0 0.2 50k)
VIN inn vcm SIN(0 -0.2 50k)
VP1 phi1 0 PULSE(0 1.8 0 1n 1n 0.45u 1u)
VP1E phi1e 0 PULSE(0 1.8 0 1n 1n 0.4u 1u)
VP2 phi2 0 PULSE(0 1.8 0.5u 1n 1n 0.45u 1u)
MS1 ap phi1 inp 0 nch W=10u L=0.18u
MS2 bn phi1 inn 0 nch W=10u L=0.18u
CS1 ap xa 1p
CS2 bn xb 1p
MS3 xa phi1e vcm 0 nch W=10u L=0.18u
MS4 xb phi1e vcm 0 nch W=10u L=0.18u
MS5 outp phi2 ap 0 nch W=10u L=0.18u
MS6 outn phi2 bn 0 nch W=10u L=0.18u
XOA xb xa outp outn fdamp
CLD outp outn 0.5p
* @RNL {
*   #XOA @SET-SYMBOL $DIFFERENTIAL_OPAMP_TWISTED;
*   # @SELECT-INSTANCE(model="nch") => @TEXT("", key="model");
*
*   /* vcm is in $BIAS_NET_SET so the bottom-plate switches get bias stubs, and the clocks are
*      unrouted so they get net-label stubs; the three sources are wired to each other on vcm, at
*      high so that the fill's unrouting of vcm does not take them */
*   &vcm @ADD-TO $BIAS_NET_SET;
*   {&phi1 &phi1e &phi2} @UNROUTE-NET(priority="high");
*   %routed_nets = # @SELECT-NET;
*   %routed_nets @ROUTE-NET;
*
*   /* what the circuit is: the pass and reset switches with their matching, the two samplers, the flip-around loop */
*   %pass_switches = {#MS1 #MS2 #MS5 #MS6} @AS $PASS_SWITCH;
*   %reset_switches = {#MS3 #MS4} @AS $RESET_SWITCH;
*   %matched_caps = {#CS1 #CS2} @AS $MATCHED_CAPACITOR;
*   %matched_input_switches = {#MS1 #MS2} @AS $MATCHED_MOS;
*   %matched_reset_switches = {#MS3 #MS4} @AS $MATCHED_MOS;
*   %matched_flip_switches = {#MS5 #MS6} @AS $MATCHED_MOS;
*   %samp_p = {#MS1 #MS3}(switch) {#CS1}(capacitor) @AS $SC_SAMPLER;
*   %samp_n = {#MS2 #MS4}(switch) {#CS2}(capacitor) @AS $SC_SAMPLER;
*   %loop = {#XOA}(forward) {#CS1 #CS2 #MS5 #MS6}(feedback) @AS $FEEDBACK_LOOP;
*   %flow = {#MS1 #MS2 #CS1 #CS2 #XOA}(path) {#VIP #VIN}(source) {#CLD}(load) @AS $SIGNAL_FLOW;
*
*   /* the flip-around switches lie with their sources on the left; the mirrors give the lower half */
*   %flip_source_orient = {#MS5:s #MS6:s} @ORIENT("left");
*
*   /* the two halves are mirror images about one horizontal axis, the amplifier and the load on it */
*   %src_mirror = #VIP(top) #VIN(bottom) #XOA(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %in_mirror = #MS1(top) #MS2(bottom) #XOA(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %cap_mirror = #CS1(top) #CS2(bottom) #XOA(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %bp_mirror = #MS3(top) #MS4(bottom) #XOA(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %fb_mirror = #MS5(top) #MS6(bottom) #XOA(axis) @AS $MIRRORED_VERTICAL_LAYOUT;
*   %center_row = {#XOA #CLD} @ALIGN("y");
*
*   /* from left to right: the sources, the input switches, the sampling capacitors, the bottom-plate switches, the
*      amplifier and the load; the flip-around switches stand over the amplifier */
*   %column_order = {%src_mirror %in_mirror %cap_mirror %bp_mirror #XOA #CLD} @ORDER("left-to-right");
*   %flip_over_amp = {#XOA %fb_mirror} @ALIGN("x");
*
*   /* the sampling circuit, with the sources, is one block */
*   %sampling_group = {#MS1 #MS2 #CS1 #CS2 #MS3 #MS4 #XOA #CLD #MS5 #MS6 #VIP #VIN} @GROUP;
*
*   /* the sources and the load stand upright; the amplifier is flipped about the x axis */
*   %upright_parts = {#VIP #VCM #VP1 #VP1E #VP2 #CLD} @ORIENT("R0");
*   %amp_orient = #XOA @ORIENT("MX");
*
*   /* the common-mode and clock sources in one row, in the order phi1, phi1e, phi2, at least 4 below the flip-around
*      switch MS6 */
*   %clock_row = {#VCM #VP1 #VP1E #VP2} @ALIGN("y") => @GROUP;
*   %clock_order = {#VCM #VP1 #VP1E #VP2} @ORDER("left-to-right");
*   %clock_distance = {{#VCM #VP1 #VP1E #VP2} #MS6} @DISTANCE("bottom-to-top", min=4);
* }
.end
