#include <iostream>
using namespace std;

int main() {
    int N;
    cin >> N;
    for (int i = 0; i < N; i++) {
        cout << -1 << (i == N - 1 ? '\n' : ' ');
    }
    return 0;
}
