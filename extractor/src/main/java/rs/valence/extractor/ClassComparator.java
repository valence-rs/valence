package rs.valence.extractor;

import java.util.Comparator;

/**
 * Compare Class objects by their simple names lexicographically.
 */
public class ClassComparator implements Comparator<Class<?>> {
    public ClassComparator() {
    }

    @Override
    public int compare(Class<?> c1, Class<?> c2) {
        return c1.getSimpleName().compareTo(c2.getSimpleName());
    }

    @Override
    public boolean equals(Object comp) {
        return comp instanceof ClassComparator;
    }
}
