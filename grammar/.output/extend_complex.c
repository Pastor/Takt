#include "extend_complex.h"

void ExtendComplex_init(ExtendComplex *main) {
    main->state = EXTEND_COMPLEX_INIT;
}

void ExtendComplex_tick(ExtendComplex *main) {
}

void ExtendComplex_reset(ExtendComplex *main) {
    ExtendComplex_init(main);
}

bool ExtendComplex_is_done(const ExtendComplex *main) {
    return main->state == EXTEND_COMPLEX_NEXT;
}

