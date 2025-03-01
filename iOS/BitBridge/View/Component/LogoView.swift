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
            LogoScene(gltfFileName: "Earth", logoScale: 1.1)
        }
    }
}

struct LogoView_Preview: PreviewProvider {
    static var previews: some View {
        LogoView(width: 200)
            .frame(width: 500, height: 500)
            .previewLayout(.sizeThatFits)
            .padding()
    }
}
