import math

fp_precision = 16
precision = 10

quarter_resolution = 1 << (precision - 2)
resolution = 1 << precision

print('pub const SIN_PRECISION: u64 = {};'.format(precision))
print('')
print('pub const SIN: [i64; {}] = ['.format(quarter_resolution + 1))
for i in range(quarter_resolution):
    angle = i / quarter_resolution / 4 * math.pi * 2
    print('    {},'.format(int(math.sin(angle) * (1 << fp_precision))))
print('    {},'.format(1 << fp_precision))
print('];')