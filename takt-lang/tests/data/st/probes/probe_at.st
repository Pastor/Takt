(* П8. VAR_GLOBAL ... AT %IX + VAR_EXTERNAL в FB.
   Подтверждает форму 0041-05 (AddressMap → AT %...). *)
VAR_GLOBAL
    Button AT %IX256.0 : BOOL;
    Lamp   AT %QX256.1 : BOOL;
END_VAR

FUNCTION_BLOCK Demo
VAR_EXTERNAL
    Button : BOOL;
    Lamp : BOOL;
END_VAR
    Lamp := Button;
END_FUNCTION_BLOCK
