package rs.valence.extractor;

import java.io.Serializable;
import java.util.Comparator;

/**
 * Compare Class objects by their simple names lexicographically.
 */
public class ClassComparator implements Comparator<Class<?>>, Serializable {
    public ClassComparator() {
    }

    @Override
    public int compare(final Class<?> c1, final Class<?> c2) {
        return c1.getSimpleName().compareTo(c2.getSimpleName());
    }

    @Override
    public boolean equals(final Object comp) {
        return comp instanceof ClassComparator;
    }
}
