#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

int counter = 0;
int counter2 = 1;
SEC("xdp")
int hello(void *ctx) {
    bpf_printk("Hello World %d", counter2);
//    bpf_printk("xxxxx yyyyy");
    counter++;
    counter2++;
    return counter2 + counter;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";