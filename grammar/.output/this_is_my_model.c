#include "this_is_my_model.h"
/// Константы, перечисления и порты модели ThisIsMyModel (ThisIsMyModel)
#define CONST_THIS_IS_MY_MODEL_NUMB 0xff
#define PORT_THIS_IS_MY_MODEL_A 0x548835
#define PORT_THIS_IS_MY_MODEL_B1 0x648835

void ThisIsMyModel_init(ThisIsMyModel *main) {
    main->state = THIS_IS_MY_MODEL_INIT;
}

void ThisIsMyModel_tick(ThisIsMyModel *main) {
}

void ThisIsMyModel_reset(ThisIsMyModel *main) {
    ThisIsMyModel_init(main);
}

bool ThisIsMyModel_is_done(const ThisIsMyModel *main) {
    return main->state == THIS_IS_MY_MODEL_ENTRY;
}

