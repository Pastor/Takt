(* П7. VAR_IN_OUT и вызов FB из FB.
   Решает О1 (0041-03): как под-FB видят переменные корня. Самый рискованный
   вопрос семантики фичи — в Takt композиция `A | B` даёт под-моделям доступ к
   портам корня в том же такте. *)
FUNCTION_BLOCK Child
VAR_IN_OUT
    shared : USINT;
END_VAR
VAR
    state : USINT := 0;
END_VAR
    shared := shared + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Parent
VAR
    value : USINT := 0;
    kid : Child;
END_VAR
    kid(shared := value);
END_FUNCTION_BLOCK
