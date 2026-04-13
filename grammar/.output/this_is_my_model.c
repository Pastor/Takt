#include "this_is_my_model.h"
#include <assert.h>
#include <math.h>
/// Константы и порты модели ThisIsMyModel (ThisIsMyModel)
#define CONST_THIS_IS_MY_MODEL_MATRIX {0, 0, 0, 0, 0, 0, 0, 0}
#define CONST_THIS_IS_MY_MODEL_NUMB 255
#define PORT_THIS_IS_MY_MODEL_A 0x548835
#define PORT_THIS_IS_MY_MODEL_B1 0x648835
/// Model functions 'Pong (ThisIsMyModel:Pong)'
static void ThisIsMyModelPong_init(ThisIsMyModel *main);
static void ThisIsMyModelPong_tick(ThisIsMyModel *main);
static void ThisIsMyModelPong_is_done(ThisIsMyModel *main);
/// Model functions 'Ping (ThisIsMyModel:Ping)'
static void ThisIsMyModelPing_init(ThisIsMyModel *main);
static void ThisIsMyModelPing_tick(ThisIsMyModel *main);
static void ThisIsMyModelPing_is_done(ThisIsMyModel *main);
/// Model functions 'Toggle (ThisIsMyModel:Toggle)'
static void ThisIsMyModelToggle_init(ThisIsMyModel *main);
static void ThisIsMyModelToggle_tick(ThisIsMyModel *main);
static void ThisIsMyModelToggle_is_done(ThisIsMyModel *main);


void ThisIsMyModel_init(ThisIsMyModel *main) {
    main->state = THIS_IS_MY_MODEL_INIT;
}

void ThisIsMyModel_tick(ThisIsMyModel *main) {
    assert(0 != main);
    switch (main->state) {
        case THIS_IS_MY_MODEL_INIT: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_ENTRY: {
            ///FIXME: Пока не реализовано
            break;
        }
        case THIS_IS_MY_MODEL_END: {
            ///FIXME: Пока не реализовано
            break;
        }
    }
}

void ThisIsMyModel_reset(ThisIsMyModel *main) {
    ThisIsMyModel_init(main);
}

bool ThisIsMyModel_is_done(const ThisIsMyModel *main) {
    return main->state == THIS_IS_MY_MODEL_ENTRY;
}

