package org.fcast.android.sender.bench;

import org.fcast.android.sender.bench.ICodecBenchmarkCallback;

interface ICodecBenchmarkService {
    void runBenchmark(String requestJson, ICodecBenchmarkCallback callback);
    void cancel();
}
