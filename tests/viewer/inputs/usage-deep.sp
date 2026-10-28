* Usages nested seven deep, with long names, and one usage that is a member of two outer usages (%analog_input_stage_wrapper_one
* is wrapped by two): the table of the usages an object is a member of keeps its rows inside a narrow panel
.model nch nmos level=1
R1 a b 1k
R2 b 0 1k
* @RNL(type="header") { @VERSION("0.3")}
* @RNL {
*   $RESISTOR_PAIR_WITH_A_LONG_DEFINITION_NAME = @DEFINE(roles=["input_side_resistor", "output_side_resistor"]) {};
*   $HIERARCHICAL_STAGE_WRAPPER_DEFINITION = @DEFINE(roles=[["wrapped_stage_usage", "usage"]]) {};
*   %analog_front_end_input_bias_network = #R1(input_side_resistor) #R2(output_side_resistor) @AS $RESISTOR_PAIR_WITH_A_LONG_DEFINITION_NAME;
*   %analog_input_stage_wrapper_one = %analog_front_end_input_bias_network(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %alternative_input_stage_wrapper = %analog_front_end_input_bias_network(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %differential_amplifier_core_wrapper_two = %analog_input_stage_wrapper_one(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %gain_stage_cascade_wrapper_three = %differential_amplifier_core_wrapper_two(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %output_buffer_chain_wrapper_four = %gain_stage_cascade_wrapper_three(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %signal_path_section_wrapper_five = %output_buffer_chain_wrapper_four(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   %complete_receiver_channel_wrapper_six = %signal_path_section_wrapper_five(wrapped_stage_usage) @AS $HIERARCHICAL_STAGE_WRAPPER_DEFINITION;
*   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0)
* }
.end
