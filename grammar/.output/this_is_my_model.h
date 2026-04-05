#ifndef THIS_IS_MY_MODEL__
#define THIS_IS_MY_MODEL__
#include <stdint.h>
#include <stdbool.h>

/** Generated 'ThisIsMyModel' structure */
struct ThisIsMyModel {
    /** Generated 'Entry_Sequence' structure */
    struct {
        /** Generated 'Ping' structure */
        struct {
            /** Generated states to 'Ping' */
            enum {
                THIS_IS_MY_MODEL_PING_INIT,
                THIS_IS_MY_MODEL_PING_START,
                THIS_IS_MY_MODEL_PING_END
            } state;
            bool toggle;
        } ping;
        /** Generated 'Pong' structure */
        struct {
            /** Generated states to 'Pong' */
            enum {
                THIS_IS_MY_MODEL_PONG_INIT,
                THIS_IS_MY_MODEL_PONG_BEGIN,
                THIS_IS_MY_MODEL_PONG_STOP
            } state;
        } pong;
        /** Generated states to 'Entry_Sequence' */
        enum {
            THIS_IS_MY_MODEL_ENTRY_SEQUENCE_INIT,
            THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP0_INIT,
            THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP0,
            THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP1_INIT,
            THIS_IS_MY_MODEL_ENTRY_SEQUENCE_STEP1
        } state;
        /** Generated 'Toggle' structure */
        struct {
            /** Generated states to 'Toggle' */
            enum {
                THIS_IS_MY_MODEL_TOGGLE_INIT,
                THIS_IS_MY_MODEL_TOGGLE_ENTRY,
                THIS_IS_MY_MODEL_TOGGLE_END,
                THIS_IS_MY_MODEL_TOGGLE_PING_INIT,
                THIS_IS_MY_MODEL_TOGGLE_PING,
                THIS_IS_MY_MODEL_TOGGLE_COMPLETE,
                THIS_IS_MY_MODEL_TOGGLE_PONG_INIT,
                THIS_IS_MY_MODEL_TOGGLE_PONG
            } state;
            /** Generated 'Ping' structure */
            struct {
                /** Generated states to 'Ping' */
                enum {
                    THIS_IS_MY_MODEL_PING_INIT,
                    THIS_IS_MY_MODEL_PING_START,
                    THIS_IS_MY_MODEL_PING_END
                } state;
                bool toggle;
            } ping;
            /** Generated 'Pong' structure */
            struct {
                /** Generated states to 'Pong' */
                enum {
                    THIS_IS_MY_MODEL_PONG_INIT,
                    THIS_IS_MY_MODEL_PONG_BEGIN,
                    THIS_IS_MY_MODEL_PONG_STOP
                } state;
            } pong;
        } toggle;
    } entry_sequence;
    /** Generated states to 'ThisIsMyModel' */
    enum {
        THIS_IS_MY_MODEL_INIT,
        THIS_IS_MY_MODEL_ENTRY_INIT,
        THIS_IS_MY_MODEL_ENTRY
    } state;
    uint64_t it;
    void *userdata;
    void  (*write_bit  )(int address, int bit, bool val, void *userdata);
    bool  (*read_bit   )(int address, int bit, void *userdata);
    void  (*write_float)(int address, int bit, float val, void *userdata);
    float (*read_float )(int address, int bit, void *userdata);
};
void ThisIsMyModel_init(struct ThisIsMyModel *main);
void ThisIsMyModel_tick(struct ThisIsMyModel *main);
void ThisIsMyModel_reset(struct ThisIsMyModel *main);
bool ThisIsMyModel_is_done(const struct ThisIsMyModel *main);
#endif
