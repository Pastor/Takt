#ifndef THIS_IS_MY_MODEL__
#define THIS_IS_MY_MODEL__
#include <stdint.h>
#include <stdbool.h>

/** Generated 'ThisIsMyModel' structure */
struct ThisIsMyModel {
    /** Generated 'Entry_Sequence' structure */
    struct {
        /** Generated 'Entry_Sequence_1Ping' structure */
        struct {
            /** Generated states to 'Entry_Sequence_1Ping' */
            enum {
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PING_INIT,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PING_START,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PING_END
            } state;
            bool toggle;
        } entry__sequence_1_ping;
        /** Generated 'Entry_Sequence_1Pong' structure */
        struct {
            /** Generated states to 'Entry_Sequence_1Pong' */
            enum {
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PONG_INIT,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PONG_STOP,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_1_PONG_BEGIN
            } state;
        } entry__sequence_1_pong;
        /** Generated states to 'Entry_Sequence' */
        enum {
            THIS_IS_MY_MODEL_ENTRY__SEQUENCE_INIT,
            THIS_IS_MY_MODEL_ENTRY__SEQUENCE_ENTRY__SEQUENCE_0_INIT,
            THIS_IS_MY_MODEL_ENTRY__SEQUENCE_ENTRY__SEQUENCE_0,
            THIS_IS_MY_MODEL_ENTRY__SEQUENCE_ENTRY__SEQUENCE_1_INIT,
            THIS_IS_MY_MODEL_ENTRY__SEQUENCE_ENTRY__SEQUENCE_1
        } state;
        /** Generated 'Entry_Sequence_0Toggle' structure */
        struct {
            /** Generated states to 'Entry_Sequence_0Toggle' */
            enum {
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_INIT,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_ENTRY,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_END,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PING_INIT,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PING,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PONG_INIT,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PONG,
                THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_COMPLETE
            } state;
            /** Generated 'PingPing' structure */
            struct {
                /** Generated states to 'PingPing' */
                enum {
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PING_PING_INIT,
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PING_PING_START,
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PING_PING_END
                } state;
                bool toggle;
            } ping_ping;
            /** Generated 'PongPong' structure */
            struct {
                /** Generated states to 'PongPong' */
                enum {
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PONG_PONG_INIT,
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PONG_PONG_BEGIN,
                    THIS_IS_MY_MODEL_ENTRY__SEQUENCE_0_TOGGLE_PONG_PONG_STOP
                } state;
            } pong_pong;
        } entry__sequence_0_toggle;
    } entry__sequence;
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
