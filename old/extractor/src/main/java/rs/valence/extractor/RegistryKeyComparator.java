package rs.valence.extractor;

import net.minecraft.registry.RegistryKey;

import java.util.Comparator;

public class RegistryKeyComparator implements Comparator<RegistryKey<?>> {
    public RegistryKeyComparator() {
    }

    @Override
    public int compare(RegistryKey<?> o1, RegistryKey<?> o2) {
        var c1 = o1.getRegistry().compareTo(o2.getRegistry());

        if (c1 != 0) {
            return c1;
        }

        return o1.getValue().compareTo(o2.getValue());
    }
}
