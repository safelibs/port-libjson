#include "config.h"

#include <stddef.h>
#include <stdio.h>

#include "arraylist.h"
#include "json_object_iterator.h"
#include "json_patch.h"
#include "json_tokener.h"
#include "json_types.h"
#include "linkhash.h"
#include "printbuf.h"

int main(void)
{
    printf("array_list\t%zu\n", sizeof(struct array_list));
    printf("lh_entry\t%zu\n", sizeof(struct lh_entry));
    printf("lh_table\t%zu\n", sizeof(struct lh_table));
    printf("printbuf\t%zu\n", sizeof(struct printbuf));
    printf("json_object_iter\t%zu\n", sizeof(struct json_object_iter));
    printf("json_object_iterator\t%zu\n", sizeof(struct json_object_iterator));
    printf("json_tokener\t%zu\n", sizeof(struct json_tokener));
    printf("json_tokener_srec\t%zu\n", sizeof(struct json_tokener_srec));
    printf("json_patch_error\t%zu\n", sizeof(struct json_patch_error));
    printf("json_object_iter.entry\t%zu\n", offsetof(struct json_object_iter, entry));
    printf("json_patch_error.errmsg\t%zu\n", offsetof(struct json_patch_error, errmsg));
    return 0;
}
