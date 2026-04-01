#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "json.h"

static int freeit_was_called = 0;
static int freeit_value = 123;
static void freeit(json_object *jso, void *userdata)
{
	(void)jso;
	(void)userdata;
	printf("freeit, value=%d\n", freeit_value);
	/* Don't actually free anything here, the userdata is stack allocated. */
	freeit_was_called = 1;
}

int main(int argc, char **argv)
{
	json_object *my_object, *my_sub_object;

	printf("Test setting, then resetting a custom serializer:\n");
	my_object = json_object_new_object();
	json_object_object_add(my_object, "abc", json_object_new_int(12));
	json_object_object_add(my_object, "foo", json_object_new_string("bar"));

	printf("my_object.to_string(standard)=%s\n", json_object_to_json_string(my_object));

	char userdata[] = "Custom Output";
	json_object_set_serializer(my_object, json_object_userdata_to_json_string, userdata, freeit);

	printf("my_object.to_string(custom serializer)=%s\n",
	       json_object_to_json_string(my_object));

	printf("Next line of output should be from the custom freeit function:\n");
	freeit_was_called = 0;
	json_object_set_serializer(my_object, NULL, NULL, NULL);
	assert(freeit_was_called);

	printf("my_object.to_string(standard)=%s\n", json_object_to_json_string(my_object));

	json_object_put(my_object);

	// ============================================

	my_object = json_object_new_object();
	printf("Check that the custom serializer isn't free'd until the last json_object_put:\n");
	json_object_set_serializer(my_object, json_object_userdata_to_json_string, userdata, freeit);
	json_object_get(my_object);
	json_object_put(my_object);
	printf("my_object.to_string(custom serializer)=%s\n",
	       json_object_to_json_string(my_object));
	printf("Next line of output should be from the custom freeit function:\n");

	freeit_was_called = 0;
	json_object_put(my_object);
	assert(freeit_was_called);

	// ============================================

	my_object = json_object_new_object();
	my_sub_object = json_object_new_double(1.0);
	json_object_object_add(my_object, "double", my_sub_object);
	printf("Check that the custom serializer does not include nul byte:\n");
#define UNCONST(a) ((void *)(uintptr_t)(const void *)(a))
	json_object_set_serializer(my_sub_object, json_object_double_to_json_string, UNCONST("%125.0f"), NULL);
	printf("my_object.to_string(custom serializer)=%s\n",
	       json_object_to_json_string_ext(my_object, JSON_C_TO_STRING_NOZERO));

	json_object_put(my_object);

	return 0;
}
