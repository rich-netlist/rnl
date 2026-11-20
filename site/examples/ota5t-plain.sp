* 5T OTA
.model nch nmos level=1 vto=0.5
.model pch pmos level=1 vto=-0.5
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4")}
.subckt ota5t inp inn out vdd vss vbias
M1 x    inp   tail vss nch W=2u L=0.5u
M2 out  inn   tail vss nch W=2u L=0.5u
M3 x    x     vdd  vdd pch W=4u L=0.5u
M4 out  x     vdd  vdd pch W=4u L=0.5u
M5 tail vbias vss  vss nch W=4u L=1u
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:a334cf047380ad3e.0457f167742baaeb") {
*   %net_label_stub_M1_g = $NET_LABEL_STUB @ATTACH #M1::g;
*   %net_label_stub_M2_g = $NET_LABEL_STUB @ATTACH #M2::g;
*   %net_label_stub_M2_d = $NET_LABEL_STUB @ATTACH #M2::d;
*   %net_label_stub_M4_d = $NET_LABEL_STUB @ATTACH #M4::d;
*   %net_label_stub_M3_s = $NET_LABEL_STUB @ATTACH #M3::s;
*   %net_label_stub_M3_b = $NET_LABEL_STUB @ATTACH #M3::b;
*   %net_label_stub_M4_s = $NET_LABEL_STUB @ATTACH #M4::s;
*   %net_label_stub_M4_b = $NET_LABEL_STUB @ATTACH #M4::b;
*   %net_label_stub_M1_b = $NET_LABEL_STUB @ATTACH #M1::b;
*   %net_label_stub_M2_b = $NET_LABEL_STUB @ATTACH #M2::b;
*   %net_label_stub_M5_s = $NET_LABEL_STUB @ATTACH #M5::s;
*   %net_label_stub_M5_b = $NET_LABEL_STUB @ATTACH #M5::b;
*   %net_label_stub_M5_g = $NET_LABEL_STUB @ATTACH #M5::g;
*   %net_label_stub_M1_d = $NET_LABEL_STUB @ATTACH #M1::d;
*   %net_label_stub_M3_d = $NET_LABEL_STUB @ATTACH #M3::d;
*   %net_label_stub_M3_g = $NET_LABEL_STUB @ATTACH #M3::g;
*   %net_label_stub_M4_g = $NET_LABEL_STUB @ATTACH #M4::g;
*   %net_label_stub_M1_s = $NET_LABEL_STUB @ATTACH #M1::s;
*   %net_label_stub_M2_s = $NET_LABEL_STUB @ATTACH #M2::s;
*   %net_label_stub_M5_d = $NET_LABEL_STUB @ATTACH #M5::d;
*   %io_port_inp = $IO_PORT @ATTACH &inp;
*   %io_port_inn = $IO_PORT @ATTACH &inn;
*   %io_port_out = $IO_PORT @ATTACH &out;
*   %io_port_vdd = $IO_PORT @ATTACH &vdd;
*   %io_port_vss = $IO_PORT @ATTACH &vss;
*   %io_port_vbias = $IO_PORT @ATTACH &vbias;
*   #M1 @PLACE(6, -2) => @ORIENT("MX");
*   #M2 @PLACE(30, 0) => @ORIENT("R0");
*   #M3 @PLACE(22, 0) => @ORIENT("R0");
*   #M4 @PLACE(38, 0) => @ORIENT("R0");
*   #M5 @PLACE(14, 0) => @ORIENT("R0");
*   %net_label_stub_M1_g @PLACE(0, 0);
*   %net_label_stub_M1_g::l @PLACE(3, -2) => @ORIENT("R0");
*   %net_label_stub_M2_g @PLACE(0, 0);
*   %net_label_stub_M2_g::l @PLACE(27, 0) => @ORIENT("R0");
*   %net_label_stub_M2_d @PLACE(0, 0);
*   %net_label_stub_M2_d::l @PLACE(30, 2) => @ORIENT("R270");
*   %net_label_stub_M4_d @PLACE(0, 0);
*   %net_label_stub_M4_d::l @PLACE(38, -2) => @ORIENT("R90");
*   %net_label_stub_M3_s @PLACE(0, 0);
*   %net_label_stub_M3_s::l @PLACE(22, 2) => @ORIENT("R270");
*   %net_label_stub_M3_b @PLACE(0, 0);
*   %net_label_stub_M3_b::l @PLACE(22, 0) => @ORIENT("MY");
*   %net_label_stub_M4_s @PLACE(0, 0);
*   %net_label_stub_M4_s::l @PLACE(38, 2) => @ORIENT("R270");
*   %net_label_stub_M4_b @PLACE(0, 0);
*   %net_label_stub_M4_b::l @PLACE(38, 0) => @ORIENT("MY");
*   %net_label_stub_M1_b @PLACE(0, 0);
*   %net_label_stub_M1_b::l @PLACE(6, -2) => @ORIENT("MY");
*   %net_label_stub_M2_b @PLACE(0, 0);
*   %net_label_stub_M2_b::l @PLACE(30, 0) => @ORIENT("MY");
*   %net_label_stub_M5_s @PLACE(0, 0);
*   %net_label_stub_M5_s::l @PLACE(14, -2) => @ORIENT("R90");
*   %net_label_stub_M5_b @PLACE(0, 0);
*   %net_label_stub_M5_b::l @PLACE(14, 0) => @ORIENT("MY");
*   %net_label_stub_M5_g @PLACE(0, 0);
*   %net_label_stub_M5_g::l @PLACE(11, 0) => @ORIENT("R0");
*   %net_label_stub_M1_d @PLACE(0, 0);
*   %net_label_stub_M1_d::l @PLACE(6, -4) => @ORIENT("R90");
*   %net_label_stub_M3_d @PLACE(0, 0);
*   %net_label_stub_M3_d::l @PLACE(22, -2) => @ORIENT("R90");
*   %net_label_stub_M3_g @PLACE(0, 0);
*   %net_label_stub_M3_g::l @PLACE(19, 0) => @ORIENT("R0");
*   %net_label_stub_M4_g @PLACE(0, 0);
*   %net_label_stub_M4_g::l @PLACE(35, 0) => @ORIENT("R0");
*   %net_label_stub_M1_s @PLACE(0, 0);
*   %net_label_stub_M1_s::l @PLACE(6, 0) => @ORIENT("R270");
*   %net_label_stub_M2_s @PLACE(0, 0);
*   %net_label_stub_M2_s::l @PLACE(30, -2) => @ORIENT("R90");
*   %net_label_stub_M5_d @PLACE(0, 0);
*   %net_label_stub_M5_d::l @PLACE(14, 2) => @ORIENT("R270");
*   %io_port_inp @PLACE(0, 23) => @ORIENT("R0");
*   %io_port_inn @PLACE(0, 19) => @ORIENT("R0");
*   %io_port_out @PLACE(0, 15) => @ORIENT("R0");
*   %io_port_vdd @PLACE(0, 11) => @ORIENT("R0");
*   %io_port_vss @PLACE(0, 7) => @ORIENT("R0");
*   %io_port_vbias @PLACE(0, 3) => @ORIENT("R0")
* }
.ends ota5t
* @RNL(type="generated", priority="high", author="rnl-solver 0.1", hash="sig2:fced5af0752234e8.53706929d5b144cf") {}
.end
