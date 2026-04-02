#include "this_is_my_model.h" 
#define PORT_THIS_IS_MY_MODEL_A 0x548835
#define PORT_THIS_IS_MY_MODEL_B1 0x648835
#define CONST_THIS_IS_MY_MODEL_M_A_T_R_I_X ({0, 0, 0, 0, 0, 0, 0, 0})
#define CONST_THIS_IS_MY_MODEL_N_U_M_B (255)
#define COND_THIS_IS_MY_MODEL_IS_EMPTY ((main->it == 0))

void ThisIsMyModel_init(struct ThisIsMyModel *main) {
    main->state = THIS_IS_MY_MODEL_INIT;
}

void ThisIsMyModel_tick(struct ThisIsMyModel *main) {
}

void ThisIsMyModel_reset(struct ThisIsMyModel *main) {
    ThisIsMyModel_init(main);
}

bool ThisIsMyModel_is_done(const struct ThisIsMyModel *main) {
    return false;
}

