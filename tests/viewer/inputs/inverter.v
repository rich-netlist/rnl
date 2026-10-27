// An inverter in Verilog
module inverter(input a, output b);
  wire c;
  assign c = ~a;
  assign b = c;
endmodule
