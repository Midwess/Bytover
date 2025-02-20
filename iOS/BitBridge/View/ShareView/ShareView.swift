//
//  ShareView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 19/2/25.
//

import SwiftUI
import Foundation

public struct ShareView: View {
    @Environment(\.safeAreaInsets) private var safeAreaInsets
    @Environment(\.screenSize) private var screenSize
    @StateObject private var core = Core()
    
    public var body: some View {
        ZStack(alignment: .bottom) {
            Theme.darkBgGradient
            Theme.starGradient(x: 0.5, y: 0.15)
            VStack {
                Spacer()
                LogoView(width: 95)
                    .frame(width: .infinity, height: 105)
                Text("Secure and Fastest \n file transfer")
                    .multilineTextAlignment(.center)
                    .modifier(Label1())
                    .foregroundStyle(Theme.gradientHeading)
                    .padding(.top, 10)
                UpgradePremiumButton()
                Spacer()
                HStack {
                    Text("Add new file")
                        .modifier(Body2())
                        .opacity(0.5)
                    AddFileButton()
                }
                Spacer()
                Spacer()
                Spacer()
                Spacer()
                Spacer()
            }
            ShareButton(width: 280)
                .padding(.bottom, 60)
        }
        .ignoresSafeArea()
    }
}

#Preview {
    ShareView()
}
