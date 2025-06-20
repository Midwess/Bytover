import SwiftUI

private struct ScreenSizeKey: EnvironmentKey {
    static var defaultValue: CGSize {
        CGSize(width: UIScreen.main.bounds.width,
               height: UIScreen.main.bounds.height)
    }
}

extension EnvironmentValues {
    var screenSize: CGSize {
        self[ScreenSizeKey.self]
    }
}
