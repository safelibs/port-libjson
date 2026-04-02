#include <json-c/json.h>

#include <string.h>

int main(void)
{
	struct json_object *value;

	value = json_tokener_parse("{\"cmake\":\"ok\"}");
	if (value == NULL)
		return 1;

	if (strcmp(json_object_get_string(json_object_object_get(value, "cmake")), "ok") != 0)
	{
		json_object_put(value);
		return 2;
	}

	json_object_put(value);
	return 0;
}
