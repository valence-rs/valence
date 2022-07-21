package dev._00a.valence_extractor;

import sun.reflect.ReflectionFactory;

public class Util {
    /**
     * Magically creates an instance of a <i>concrete</i> class without calling its constructor.
     */
    public static <T> T magicallyInstantiate(Class<T> clazz) {
        var rf = ReflectionFactory.getReflectionFactory();
        try {
            var objCon = Object.class.getDeclaredConstructor();
            var con = rf.newConstructorForSerialization(clazz, objCon);
            return clazz.cast(con.newInstance());
        } catch (Throwable e) {
            throw new IllegalArgumentException("Failed to magically instantiate " + clazz.getName(), e);
        }
    }
}
