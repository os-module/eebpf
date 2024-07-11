#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

int counter = 0;

SEC("xdp")
int hello(void *ctx) {
    bpf_printk("Hello World %d", 10);
    bpf_printk("xxxxx yyyyy");
    counter++;
    return XDP_PASS;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";