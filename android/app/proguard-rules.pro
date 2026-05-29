-keep class com.soundbridge.native.NativeAudioEngine { *; }
-keep class com.soundbridge.audio.** { *; }

-keepclasseswithmembernames class * {
    native <methods>;
}

-keepattributes *Annotation*
-keepattributes SourceFile,LineNumberTable
-keepattributes Signature
-keepattributes Exceptions
