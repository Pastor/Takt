(* П4. Перечислимый TYPE с явными значениями.
   Решает T13 (0041-02): прямое отображение enum Takt → TYPE, или откат
   Option C (USINT + VAR_GLOBAL CONSTANT). *)
TYPE
    Floor : (Bottom := 80, Top);
END_TYPE

FUNCTION_BLOCK Demo
VAR
    f : Floor := Bottom;
END_VAR
    IF f = Bottom THEN
        f := Top;
    END_IF;
END_FUNCTION_BLOCK
