package rs.valence.extractor;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import java.util.TreeMap;

/**
 * Utility class for various methods.
 */
public class ValenceUtils {
    private ValenceUtils() {
        throw new UnsupportedOperationException("This class cannot be instantiated");
    }

    /**
     * Converts a string to PascalCase.
     * 
     * @param str The string to convert.
     * @return The converted string.
     */
    public static String toPascalCase(String str) {
        StringBuilder result = new StringBuilder();
        boolean capitalizeNext = true;

        for (char c : str.toCharArray()) {
            if (Character.isWhitespace(c) || c == '_') {
                capitalizeNext = true;
            } else if (capitalizeNext) {
                result.append(Character.toUpperCase(c));
                capitalizeNext = false;
            } else {
                result.append(Character.toLowerCase(c));
            }
        }

        return result.toString();
    }

    /**
     * Converts a TreeMap to a JsonArray, ignoring the keys.
     */
    public static <A, B extends JsonElement> JsonArray treeMapToJsonArray(TreeMap<A, B> map) {
        JsonArray array = new JsonArray();
        for (var entry : map.entrySet()) {
            array.add(entry.getValue());
        }
        return array;
    }
}
