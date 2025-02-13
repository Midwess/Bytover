//
//  LogoView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct LogoView: View {
    let width: CGFloat
    var body: some View {
        VStack {
            LogoScene()
        }
    }
}

struct LogoView_Preview: PreviewProvider {
    static var previews: some View {
        LogoView(width: 200)
            .frame(width: 300, height: 100)
            .previewLayout(.sizeThatFits)
            .padding()
    }
}
